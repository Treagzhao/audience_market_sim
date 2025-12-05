use std::collections::HashMap;
use rand::Rng;
use rand::seq::SliceRandom;
use crate::model::agent::Agent;
use crate::model::factory::Factory;
use crate::model::product::Product;
use crate::logging::log_trade;
use std::sync::Arc;
use std::sync::RwLock;
use rayon::prelude::*;

pub struct Market{
    factories:HashMap<u64,Arc<RwLock<Vec<Factory>>>>,
    products:Vec<Product>,
    agents:Arc<RwLock<Vec<Agent>>>,
}

impl Market{
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
                    product
                );
                product_factories.push(factory);
                factory_id_counter += 1;
            }
            
            // 将工厂列表包装为Arc<RwLock<Vec<Factory>>>
            factories.insert(product.id(), Arc::new(RwLock::new(product_factories)));
        }
        
        // 生成10000个消费者，每个消费者初始有10万块钱
        for agent_id in 1..=10000 {
            let agent = Agent::new(
                agent_id,
                format!("Consumer_{}", agent_id),
                100000.0
            );
            agents_vec.push(agent);
        }
        
        Market {
            factories,
            products,
            agents: Arc::new(RwLock::new(agents_vec)),
        }
    }
    
    /// 处理单个商品的交易逻辑（线程安全版本）
    fn process_product_trades(&self, round: u64, product_id: u64) -> u64 {
        let mut trades_count = 0;
        
        // 查找产品
        if let Some(product) = self.products.iter().find(|p| p.id() == product_id) {
            // 获取工厂列表的Arc副本
            if let Some(factory_list_arc) = self.factories.get(&product_id) {
                // 克隆Arc，以便在闭包中使用
                let factory_list_arc_clone = factory_list_arc.clone();
                let agents_clone = self.agents.clone();
                let product_clone = product.clone();
                
                // 在闭包中处理工厂交易
                let local_trades = rayon::scope(|_s| {
                    let mut local_count = 0;
                    
                    // 获取工厂列表的读写锁
                    let mut factory_list = factory_list_arc_clone.write().unwrap();
                    
                    // 遍历商品下的工厂
                    for factory in &mut *factory_list {
                        // 让工厂开启一次循环
                        factory.start_round(round);
                        
                        // 获取agents的可变锁
                        let mut agents = agents_clone.write().unwrap();
                        
                        // 让每个agent与工厂进行交易
                        for agent in &mut *agents {
                            // 检查工厂库存，如果为0则退出循环
                            if factory.get_stock(round) <= 0 {
                                break;
                            }
                            
                            // 调用agent的trade方法
                            let trade_result = agent.trade(factory);
                            
                            // 调用工厂的deal方法
                            factory.deal(&trade_result, round);
                            
                            // 如果交易成功，增加交易计数
                            if matches!(trade_result, crate::model::agent::TradeResult::Success(_)) {
                                local_count += 1;
                            }
                            
                            // 记录交易日志
                            if let Err(e) = log_trade(round, agent, factory, &product_clone, &trade_result) {
                                eprintln!("Failed to log trade: {}", e);
                            }
                        }
                    }
                    
                    local_count
                });
                
                trades_count = local_trades;
            }
        }
        
        trades_count
    }
    

    
    pub fn run(&mut self) {
        let mut rng = rand::thread_rng();
        let mut round = 1;
        let mut total_trades = 0;
        const MAX_ROUND: u64 = 100;
        
        loop {
            println!("Starting round {}, Total trades: {}", round, total_trades);
            
            // 打乱所有工厂的顺序
            for (_product_id, factory_list_arc) in self.factories.iter_mut() {
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
            
            // 使用rayon并行处理产品交易
            let parallel_trades: Vec<u64> = product_ids.par_iter().map(|&product_id| {
                // 处理单个商品的交易
                self.process_product_trades(round, product_id)
            }).collect();
            
            // 汇总本轮交易数
            let round_trades: u64 = parallel_trades.iter().sum();
            total_trades += round_trades;
            
            // 检查是否所有agent的余额为0
            let agents = self.agents.read().unwrap();
            let all_agents_broke = agents.iter().all(|agent| agent.cash() <= 0.0);
            
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
        }
    }
}