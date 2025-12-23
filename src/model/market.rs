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
const MAX_ROUND: u64 = 8000;
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

    fn shuffle_before_round(&mut self) {
        let mut rng = rand::thread_rng();
        let mut factories = self.factories.clone();
        for (_product_id, factory_list_arc) in factories.iter_mut() {
            let mut factory_list = factory_list_arc.write();
            factory_list.shuffle(&mut rng);
        }
        let mut agents = self.agents.write();
        agents.shuffle(&mut rng);
    }

    fn set_agent_log_after_round(&mut self, round: u64, timestamp: i64, total_trades: u64) {
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
    }

    fn factory_log_after_round(&mut self, round: u64, timestamp: i64, total_trades: u64) {
        for (_product_id, factory_list_arc) in self.factories.iter() {
            let factory_list = factory_list_arc.read();
            for factory in factory_list.iter() {
                let product_id = factory.product_id();
                let (supply_range_lower, supply_range_upper) = factory.supply_price_range();
                // 获取本轮财务账单
                let bill = factory.get_round_bill(round);
                // 计算毛利率
                let gross_margin = bill.get_cogs();
                // 获取工厂状态
                let factory_status = format!("{:?}", factory.status());
                let mut logger = LOGGER.write();
                if let Err(e) = logger.log_factory_end_of_round(
                    timestamp,
                    round,
                    factory.id(),
                    factory.name().to_string(),
                    product_id,
                    format!("{:?}", factory.product_category()),
                    factory.cash(),
                    bill.initial_stock,
                    bill.remaining_stock,
                    supply_range_lower,
                    supply_range_upper,
                    // 新增财务字段数据
                    bill.units_sold,
                    bill.revenue,
                    bill.total_stock,
                    bill.total_production,
                    bill.rot_stock,
                    bill.production_cost,
                    bill.profit,
                    // 新增毛利率数据
                    gross_margin,
                    // 新增工厂状态数据
                    factory_status,
                ) {
                    eprintln!("Failed to log factory end of round: {}", e);
                }
            }
        }
    }

    fn ubi(&mut self) {
        let mut agents = self.agents.write();
        agents.iter_mut().for_each(|agent| {
            let mut a = agent.write();
            a.income((800.0, 1200.0));
        });
    }

    pub fn run(&mut self) {
        let mut rng = rand::thread_rng();
        let mut round = 1; //比如得从1 开始，因为很多初值是以0来设置的
        let mut total_trades = 0;

        loop {
            println!("Starting round {}, Total trades: {}", round, total_trades);
            let current_timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as i64;
            self.shuffle_before_round();
            let factories = &self.factories;
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
            for (_product_id, factory_list_arc) in self.factories.iter_mut() {
                let mut factory_list = factory_list_arc.write();
                for factory in factory_list.iter_mut() {
                    factory.settling_after_round(round);
                }
            }
            self.set_agent_log_after_round(round, current_timestamp, total_trades);
            self.factory_log_after_round(round, current_timestamp, total_trades);

            self.ubi();
            if self.break_simulation_loop(round, self.consecutive_zero_trades) {
                break;
            }
            round += 1;
            thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    fn break_simulation_loop(&self, round: u64, consecutive_zero_trades: u32) -> bool {
        let agents = self.agents.read();
        let all_agents_broke_up = agents.iter().all(|agent| {
            let mut a = agent.read();
            a.cash() < 0.01
        });
        if all_agents_broke_up {
            println!("simulation finish at round {}", round);
            println!("Reason: All agents have zero or negative cash.\n");
            return true;
        }
        if round > MAX_ROUND {
            println!("simulation finish at round {}", round);
            println!("Reason: Reached maximum rounds ({})\n", MAX_ROUND);
            return true;
        }
        if consecutive_zero_trades >= 20 {
            println!("simulation finish at round {}", round);
            println!(
                "Reason: No trades for {} consecutive rounds.\n",
                consecutive_zero_trades
            );
            return true;
        }
        false
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::normal_distribute::NormalDistribution;
    use crate::model::product::ProductCategory;

    // 测试 shuffle_before_round 方法
    #[test]
    fn test_shuffle_before_round() {
        // 创建一个简单的产品用于测试
        let price_distribution =
            NormalDistribution::new(100.0, 1, "test_price_dist".to_string(), 10.0);
        let elastic_distribution =
            NormalDistribution::new(1.0, 1, "test_elastic_dist".to_string(), 0.2);
        let cost_distribution = NormalDistribution::new(80.0, 1, "test_cost_dist".to_string(), 5.0);

        let product = Product::from(
            1,
            "Test Product".to_string(),
            ProductCategory::from_str("Food"),
            1.0,
            price_distribution,
            elastic_distribution,
            cost_distribution,
        );

        let products = vec![product];

        // 创建市场实例
        let mut market = Market::new(products);

        // 获取初始状态
        let initial_factories = market.factories.clone();
        let initial_agents = market.agents.read().clone();

        // 获取初始工厂顺序（按ID）
        let initial_factory_ids: Vec<u64> = initial_factories
            .get(&1)
            .unwrap()
            .read()
            .iter()
            .map(|f| f.id())
            .collect();

        // 获取初始agent顺序（按ID）
        let initial_agent_ids: Vec<u64> = initial_agents.iter().map(|a| a.read().id()).collect();

        // 多次调用 shuffle_before_round 方法，提高顺序变化的概率
        let mut factory_shuffled = false;
        let mut agent_shuffled = false;

        // 最多尝试10次，直到顺序发生变化
        for _ in 0..10 {
            market.shuffle_before_round();

            // 获取打乱后的状态
            let current_factory_ids: Vec<u64> = market
                .factories
                .get(&1)
                .unwrap()
                .read()
                .iter()
                .map(|f| f.id())
                .collect();

            let current_agent_ids: Vec<u64> =
                market.agents.read().iter().map(|a| a.read().id()).collect();

            if initial_factory_ids != current_factory_ids {
                factory_shuffled = true;
            }

            if initial_agent_ids != current_agent_ids {
                agent_shuffled = true;
            }

            // 如果两者都已经变化，就可以提前结束
            if factory_shuffled && agent_shuffled {
                break;
            }
        }

        // 验证工厂和agent的顺序至少有一次发生了变化
        assert!(factory_shuffled, "经过10次尝试，工厂顺序没有被打乱");
        assert!(agent_shuffled, "经过10次尝试，agent顺序没有被打乱");

        // 最后一次获取打乱后的状态，用于验证数量和ID存在性
        let final_factory_ids: Vec<u64> = market
            .factories
            .get(&1)
            .unwrap()
            .read()
            .iter()
            .map(|f| f.id())
            .collect();

        let final_agent_ids: Vec<u64> =
            market.agents.read().iter().map(|a| a.read().id()).collect();

        // 验证所有工厂和agent都被保留，只是顺序变化
        assert_eq!(
            initial_factory_ids.len(),
            final_factory_ids.len(),
            "工厂数量发生变化"
        );
        assert_eq!(
            initial_agent_ids.len(),
            final_agent_ids.len(),
            "agent数量发生变化"
        );

        // 验证所有原始ID都存在于打乱后的列表中
        for id in initial_factory_ids.iter() {
            assert!(final_factory_ids.contains(id), "工厂ID {} 丢失", id);
        }

        for id in initial_agent_ids.iter() {
            assert!(final_agent_ids.contains(id), "agent ID {} 丢失", id);
        }
    }

    // 测试 break_simulation_loop 方法
    #[test]
    fn test_break_simulation_loop() {
        // 创建一个简单的产品用于测试
        let price_distribution =
            NormalDistribution::new(100.0, 1, "test_price_dist".to_string(), 10.0);
        let elastic_distribution =
            NormalDistribution::new(1.0, 1, "test_elastic_dist".to_string(), 0.2);
        let cost_distribution = NormalDistribution::new(80.0, 1, "test_cost_dist".to_string(), 5.0);

        let product = Product::from(
            1,
            "Test Product".to_string(),
            ProductCategory::from_str("Food"),
            1.0,
            price_distribution,
            elastic_distribution,
            cost_distribution,
        );

        let products = vec![product];

        // 创建市场实例，使用products.clone()保留原始products
        let market = Market::new(products.clone());

        // 测试1: 没有退出条件满足时，返回false
        let result1 = market.break_simulation_loop(100, 5);
        assert!(
            !result1,
            "当没有退出条件满足时，break_simulation_loop 应返回 false"
        );

        // 测试2: 当连续零交易量达到20时，返回true
        let result2 = market.break_simulation_loop(100, 20);
        assert!(
            result2,
            "当连续零交易量达到20时，break_simulation_loop 应返回 true"
        );

        // 测试3: 当达到最大轮次时，返回true
        const MAX_ROUND: u64 = 8000;
        let result3 = market.break_simulation_loop(MAX_ROUND + 1, 5);
        assert!(
            result3,
            "当达到最大轮次时，break_simulation_loop 应返回 true"
        );

        // 测试4: 当所有代理人破产时，返回true
        // 创建一个所有代理人都破产的市场
        let market_with_broke_agents = Market::new(products);
        {
            let mut agents = market_with_broke_agents.agents.write();
            for agent in agents.iter_mut() {
                let mut a = agent.write();
                // 将代理人的现金设置为0，使其破产
                a.set_cash(0.0);
            }
        }

        let result4 = market_with_broke_agents.break_simulation_loop(100, 5);
        assert!(
            result4,
            "当所有代理人破产时，break_simulation_loop 应返回 true"
        );
    }

    // 测试 ubi 方法
    #[test]
    fn test_ubi() {
        // 创建一个简单的产品用于测试
        let price_distribution =
            NormalDistribution::new(100.0, 1, "test_price_dist".to_string(), 10.0);
        let elastic_distribution =
            NormalDistribution::new(1.0, 1, "test_elastic_dist".to_string(), 0.2);
        let cost_distribution = NormalDistribution::new(80.0, 1, "test_cost_dist".to_string(), 5.0);

        let product = Product::from(
            1,
            "Test Product".to_string(),
            ProductCategory::from_str("Food"),
            1.0,
            price_distribution,
            elastic_distribution,
            cost_distribution,
        );

        let products = vec![product];

        // 创建市场实例
        let mut market = Market::new(products);

        // 记录初始现金
        let initial_cash: Vec<f64> = {
            let agents = market.agents.read();
            agents.iter().map(|agent| agent.read().cash()).collect()
        };

        // 调用 ubi 方法
        market.ubi();

        // 记录调用后的现金
        let after_cash: Vec<f64> = {
            let agents = market.agents.read();
            agents.iter().map(|agent| agent.read().cash()).collect()
        };

        // 验证所有代理人的现金都有所增加，且增加的金额在预期范围内（800.0 到 1200.0 之间）
        for (i, (initial, after)) in initial_cash.iter().zip(after_cash.iter()).enumerate() {
            let increase = after - initial;
            assert!(
                increase >= 800.0,
                "代理人 {} 的收入增加量不应少于 800.0，实际增加了 {}",
                i + 1,
                increase
            );
            assert!(
                increase <= 1200.0,
                "代理人 {} 的收入增加量不应超过 1200.0，实际增加了 {}",
                i + 1,
                increase
            );
        }
    }
}
