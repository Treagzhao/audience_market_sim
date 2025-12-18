use crate::logging::LOGGER;
use crate::model::agent::{Agent, TradeResult};
use crate::model::factory::Factory;
use crate::model::product::Product;
use parking_lot::RwLock;
use rand::Rng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Market {
    factories: HashMap<u64, Arc<RwLock<Vec<Factory>>>>,
    products: Vec<Product>,
    agents: Arc<RwLock<Vec<Arc<RwLock<Agent>>>>>,
    consecutive_zero_trades: u32, // 跟踪连续0成交量的轮次数
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
                10000.0,
                &products,
                true,
            );
            agents_vec.push(Arc::new(RwLock::new(agent)));
        }
        println!("after agents created");
        Market {
            factories,
            products,
            agents: Arc::new(RwLock::new(agents_vec)),
            consecutive_zero_trades: 0, // 初始化连续0成交量轮次为0
        }
    }

    pub fn run(&mut self) {
        let mut rng = rand::thread_rng();
        let mut round = 1;
        let mut total_trades = 0;
        const MAX_ROUND: u64 = 8000;

        loop {
            println!("Starting round {}, Total trades: {}", round, total_trades);
            let current_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as i64;
            let mut factories = self.factories.clone();
            // 打乱所有工厂的顺序
            for (_product_id, factory_list_arc) in factories.iter_mut() {
                let mut factory_list = factory_list_arc.write();
                factory_list.shuffle(&mut rng);
            }

            // 打乱所有消费者的顺序
            {
                let mut agents = self.agents.write();
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
                    println!("dealing product :{:?}", product_id);
                    let count = process_product_trades(
                        current_timestamp,
                        products,
                        f_list,
                        agents,
                        round,
                        product_id,
                    );
                    let mut c = counter.write();
                    *c += count;
                });
                handles.push(h);
            }

            // 等待所有线程完成
            for h in handles {
                h.join().expect("error ");
            }

            // 汇总本轮交易数
            let current_round_trades = {
                let r = round_trades.read();
                *r
            };
            total_trades += current_round_trades;

            // 更新连续0成交量轮次计数
            if current_round_trades == 0 {
                self.consecutive_zero_trades += 1;
            } else {
                self.consecutive_zero_trades = 0;
            }

            // 记录每个agent的现金情况
            {
                // 生成一个共享的timestamp，确保同一轮的所有log_cash事件使用同一个时间戳
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Failed to get system time")
                    .as_millis() as i64;

                let agents = self.agents.read();
                for agent in agents.iter() {
                    let a = agent.read();
                    let mut logger = LOGGER.write();
                    if let Err(e) = logger.log_agent_cash(
                        timestamp,
                        round,
                        a.id(),
                        a.name().to_string(),
                        a.cash(),
                        total_trades,
                    ) {
                        eprintln!("Failed to log agent cash: {}", e);
                    }
                }

                // 记录每个工厂的轮次结束日志，使用与agent现金日志相同的timestamp
                for (_product_id, factory_list_arc) in self.factories.iter() {
                    let factory_list = factory_list_arc.read();
                    for factory in factory_list.iter() {
                        let product_id = factory.product_id();
                        let (supply_range_lower, supply_range_upper) = factory.supply_price_range();
                        // 获取本轮开始时的初始产量
                        let initial_stock = factory.get_initial_stock();
                        // 获取本轮结束时的剩余库存（经过交易后的库存）
                        let remaining_stock = factory.get_stock(round);
                        let mut logger = LOGGER.write();
                        if let Err(e) = logger.log_factory_end_of_round(
                            timestamp,
                            round,
                            factory.id(),
                            factory.name().to_string(),
                            product_id,
                            format!("{:?}", factory.product_category()),
                            factory.cash(),
                            initial_stock,
                            remaining_stock,
                            supply_range_lower,
                            supply_range_upper,
                        ) {
                            eprintln!("Failed to log factory end of round: {}", e);
                        }
                    }
                }
            }

            // 检查是否所有agent的余额为0
            let all_agents_broke = {
                let agents = self.agents.write();
                agents.iter().all(|agent| {
                    let mut a = agent.write();
                    a.income((800.0, 1200.0));
                    a.cash() < 0.01
                })
            };

            // 检查退出条件
            if round > MAX_ROUND || all_agents_broke || self.consecutive_zero_trades >= 20 {
                println!("Simulation ending...");
                if round > MAX_ROUND {
                    println!("Reason: Reached maximum rounds ({})\n", MAX_ROUND);
                }
                if all_agents_broke {
                    println!("Reason: All agents have zero or negative cash.\n");
                }
                if self.consecutive_zero_trades >= 20 {
                    println!(
                        "Reason: No trades for {} consecutive rounds.\n",
                        self.consecutive_zero_trades
                    );
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
    timestamp: i64,
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
    let mut factory_list_arc = factories.write();
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
            let mut agents = agents_clone.read();
            println!(
                "current dealing product :{:?} factory_id:{:?}",
                product_id,
                factory.id()
            );
            // 让每个agent与工厂进行交易
            for a in agents.iter() {
                let (agent_id, agent_name) = {
                    let agent = a.read();
                    (agent.id(), agent.name().to_string())
                };

                // 检查工厂库存，如果为0则退出循环
                if factory.get_stock(round) <= 0 {
                    break;
                }
                let has_demand = {
                    let agent = a.read();
                    agent.has_demand(product_id)
                };
                let mut trade_result = TradeResult::NotYet;
                let mut interval_relation = None;
                if !has_demand {
                    trade_result = TradeResult::NotMatched;
                } else {
                    let mut agent = a.write();
                    // 调用agent的trade方法
                    (trade_result, interval_relation) = agent.trade(factory, round);
                    drop(agent);
                }
                // 将interval_relation转换为字符串
                let interval_relation_str = match &interval_relation {
                    Some(rel) => match rel {
                        crate::model::agent::IntervalRelation::Overlapping(_) => "Overlapping",
                        crate::model::agent::IntervalRelation::AgentBelowFactory => {
                            "AgentBelowFactory"
                        }
                        crate::model::agent::IntervalRelation::AgentAboveFactory => {
                            "AgentAboveFactory"
                        }
                    },
                    None => "None",
                };

                // 调用工厂的deal方法
                factory.deal(&trade_result, round, interval_relation);

                // 如果交易成功，增加交易计数
                if matches!(trade_result, crate::model::agent::TradeResult::Success(_)) {
                    local_count += 1;
                }
                let (
                    agent_cash,
                    agent_pref_original_price,
                    agent_pref_original_elastic,
                    agent_pref_current_price,
                    agent_pref_current_range_lower,
                    agent_pref_current_range_upper,
                ) = {
                    let agent = a.read();
                    let preferences_map = agent.preferences();
                    let preferences = preferences_map.get(&product.product_category()).unwrap();
                    if let Some(x) = preferences.get(&product_id) {
                        (
                            agent.cash(),
                            x.original_price,
                            x.original_elastic,
                            x.current_price,
                            x.current_range.0,
                            x.current_range.1,
                        )
                    } else {
                        (agent.cash(), 0.0, 0.0, 0.0, 0.0, 0.0)
                    }
                };
                // 记录交易日志
                let mut logger = LOGGER.write();
                if let Err(e) = logger.log_trade(
                    timestamp,
                    round,
                    0,
                    agent_id,
                    agent_name,
                    agent_cash,
                    agent_pref_original_price,
                    agent_pref_original_elastic,
                    agent_pref_current_price,
                    agent_pref_current_range_lower,
                    agent_pref_current_range_upper,
                    factory,
                    product,
                    &trade_result,
                    interval_relation_str,
                ) {
                    eprintln!("Failed to log trade: {}", e);
                }
            }
        }

        local_count
    };

    trades_count = local_trades;

    trades_count
}
