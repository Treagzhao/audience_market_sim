use crate::logging::log_trade;
use crate::model::agent::{Agent, TradeResult};
use crate::model::factory::Factory;
use crate::model::product::Product;
use rand::Rng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::thread::JoinHandle;

pub struct Market {
    factories: HashMap<u64, Arc<RwLock<Vec<Factory>>>>,
    products: Vec<Product>,
    agents: Arc<RwLock<Vec<Arc<RwLock<Agent>>>>>,
}

impl Market {
    pub fn new(products: Vec<Product>) -> Self {
        let mut factories = HashMap::new();
        let mut agents_vec = Vec::new();
        let mut rng = rand::thread_rng();
        let mut factory_id_counter = 1;

        // 为每个产品创建3或4个工厂
        for product in &products {
            let factory_count = if rng.gen_bool(0.5) { 3 } else { 4 };
            let mut product_factories = Vec::with_capacity(factory_count);

            for i in 0..factory_count {
                let factory = Factory::new(
                    factory_id_counter,
                    format!("{}_{}", product.name(), i),
                    product,
                );
                product_factories.push(factory);
                factory_id_counter += 1;
            }

            // 将工厂列表包装为Arc<RwLock<Vec<Factory>>>
            factories.insert(product.id(), Arc::new(RwLock::new(product_factories)));
        }
        println!("before agent created");
        // 生成100个消费者，每个消费者初始有10万块钱
        for agent_id in 1..=100 {
            let agent = Agent::new(
                agent_id,
                format!("Consumer_{}", agent_id),
                100.0,
                &products,
            );
            agents_vec.push(Arc::new(RwLock::new(agent)));
        }
        println!("after agents created");
        Market {
            factories,
            products,
            agents: Arc::new(RwLock::new(agents_vec)),
        }
    }

    pub fn run(&mut self) {
        let mut rng = rand::thread_rng();
        let mut round = 1;
        let mut total_trades = 0;
        const MAX_ROUND: u64 = 2000;

        loop {
            println!("Starting round {}, Total trades: {}", round, total_trades);
            let mut factories = self.factories.clone();
            // 打乱所有工厂的顺序
            for (_product_id, factory_list_arc) in factories.iter_mut() {
                let mut factory_list = factory_list_arc.write().unwrap();
                factory_list.shuffle(&mut rng);
            }

            // 打乱所有消费者的顺序
            {
                let mut agents = self.agents.write().unwrap();
                agents.shuffle(&mut rng);
            }

            // 获取产品ID列表
            let product_ids: Vec<u64> = self.products.iter().map(|p| p.id()).collect();
            let mut handles: Vec<JoinHandle<_>> = Vec::new();
            let round_trades: Arc<RwLock<u64>> = Arc::new(RwLock::new(0));
            for i in 0..product_ids.len() {
                let product_id = product_ids[i];
                let products = self.products.clone();
                let f = factories.get(&product_id);
                if f.is_none() {
                    continue;
                }
                let f_list = f.unwrap().clone();
                let agents = self.agents.clone();
                let mut counter = round_trades.clone();
                let h = thread::spawn(move || {
                    let count = process_product_trades(products, f_list, agents, round, product_id);
                    let mut c = counter.write().unwrap();
                    *c += count;
                });
                handles.push(h);
            }

            // 等待所有线程完成
            for h in handles {
                h.join().expect("error ");
            }

            // 汇总本轮交易数
            total_trades += {
                let r = round_trades.read().unwrap();
                *r
            };

            // 检查是否所有agent的余额为0
            let agents = self.agents.read().unwrap();
            let all_agents_broke = agents.iter().all(|agent| {
                let a = agent.read().unwrap();
                a.cash() < 0.01
            });

            // 检查退出条件
            if round > MAX_ROUND || all_agents_broke {
                println!("Simulation ending...");
                if round > MAX_ROUND {
                    println!("Reason: Reached maximum rounds ({}).", MAX_ROUND);
                }
                if all_agents_broke {
                    println!("Reason: All agents have zero or negative cash.");
                }
                break;
            }

            round += 1;
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

/// 处理单个商品的交易逻辑（线程安全版本）
fn process_product_trades(
    products: Vec<Product>,
    factories: Arc<RwLock<Vec<Factory>>>,
    agents: Arc<RwLock<Vec<Arc<RwLock<Agent>>>>>,
    round: u64,
    product_id: u64,
) -> u64 {
    println!("dealing:{:?}", product_id);
    let mut trades_count = 0;
    let p = products.iter().find(|p| p.id() == product_id);
    if p.is_none() {
        return 0;
    }
    let product = p.unwrap();
    // 查找产品
    // 获取工厂列表的Arc副本
    let mut factory_list_arc = factories.write().unwrap();
    // 克隆Arc，以便在闭包中使用
    let factory_list_arc_clone = factory_list_arc;
    let agents_clone = agents.clone();
    let product_clone = product.clone();

    // 在闭包中处理工厂交易
    let local_trades = {
        let mut local_count = 0;

        // 获取工厂列表的读写锁
        let mut factory_list = factory_list_arc_clone;

        // 遍历商品下的工厂
        for factory in factory_list.iter_mut() {
            // 让工厂开启一次循环
            factory.start_round(round);

            // 获取agents的可变锁
            let mut agents = agents_clone.read().unwrap();

            // 让每个agent与工厂进行交易
            for a in agents.iter() {
                // 检查工厂库存，如果为0则退出循环
                if factory.get_stock(round) <= 0 {
                    break;
                }
                let has_demand = {
                    let agent = a.read().unwrap();
                    agent.has_demand(product_id)
                };
                let mut trade_result = TradeResult::NotYet;
                if !has_demand {
                    trade_result = TradeResult::NotMatched;
                } else {
                    let mut agent = a.write().unwrap();
                    // 调用agent的trade方法
                    trade_result = agent.trade(factory);
                }
                // 调用工厂的deal方法
                factory.deal(&trade_result, round);

                // 如果交易成功，增加交易计数
                if matches!(trade_result, crate::model::agent::TradeResult::Success(_)) {
                    local_count += 1;
                }

                // 记录交易日志
                if let Err(e) = log_trade(round, a.clone(), factory, &product_clone, &trade_result)
                {
                    eprintln!("Failed to log trade: {}", e);
                }
            }
        }

        local_count
    };

    trades_count = local_trades;

    trades_count
}
