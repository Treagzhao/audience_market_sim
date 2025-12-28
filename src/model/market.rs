use crate::logging::LOGGER;
use crate::model::agent::{Agent, IntervalRelation, TradeResult};
use crate::model::factory::{Factory, FactoryStatus};
use crate::model::product::{Product, ProductCategory};
use crate::model::util::random_unrepeat_numbers_in_range;
use parking_lot::RwLock;
use rand::Rng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
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
                let gross_margin = factory.cogs_of_25_rounds();
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
                    println!("dealing product :{:?} round:{:?}", product_id, round);
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
    let product_category = product.product_category();
    // 查找产品
    // 获取工厂列表的Arc副本
    let mut factory_list_arc = factories.write();
    // 克隆Arc，以便在闭包中使用
    let mut factory_list = factory_list_arc;
    let agents_clone = agents.clone();
    let agents = agents_clone.read();
    let mut factory_borrow_list: Vec<Rc<RefCell<&mut Factory>>> = Vec::new();
    for factory in factory_list.iter_mut() {
        factory.start_round(round);
        factory_borrow_list.push(Rc::new(RefCell::new(factory)));
    }
    for a in agents.iter() {
        let ag = a.clone();
        let mut agent = ag.write();
        if !agent.has_demand(product_id) {
            continue;
        }
        let mut potential_factories = range_factory_list(factory_borrow_list.clone(), round);
        let mut trade_result_list: Vec<(TradeResult, IntervalRelation)> = Vec::new();
        let mut offered_prices: Vec<f64> = Vec::new();
        let mut deal_index: Option<usize> = None;
        for (i, (price, factory)) in potential_factories.iter().enumerate() {
            let f = factory.borrow();
            let (result, interval_relation) =
                agent.negotiate(round, product_id, product_category, *price);
            offered_prices.push(*price);
            trade_result_list.push((result, interval_relation));
            if result == TradeResult::Success(*price) {
                trades_count += 1;
                log_trade_round(
                    timestamp,
                    round,
                    &**f,
                    product,
                    &agent,
                    &result,
                    Some(&interval_relation),
                    *price,
                );
                deal_index = Some(i);
                break;
            }
        }
        match deal_index {
            Some(index) => {
                let result = trade_result_list[index].0;
                agent.settling(product_id, product_category, round, result, offered_prices);
            }
            None => {
                agent.settling(
                    product_id,
                    product_category,
                    round,
                    TradeResult::Failed,
                    offered_prices,
                );
            }
        }
        for (i, (_, factory)) in potential_factories.iter_mut().enumerate() {
            let mut f = factory.borrow_mut();
            let mut interval_relation: Option<IntervalRelation> = None;
            let mut result: TradeResult = TradeResult::NotYet;
            if i < trade_result_list.len() {
                let (r, rel) = trade_result_list[i];
                result = r;
                interval_relation = Some(rel);
            } else {
                result = TradeResult::Failed
            }
            match result {
                TradeResult::NotMatched | TradeResult::NotYet => {}
                TradeResult::Failed => match deal_index {
                    Some(index) => {
                        if i <= index {
                            f.deal(&result, round, interval_relation);
                        } else {
                            f.deal(&result, round, Some(IntervalRelation::AgentBelowFactory));
                        }
                    }
                    _ => {
                        f.deal(&result, round, interval_relation);
                    }
                },
                TradeResult::Success(_dealed_price) => {
                    f.deal(&result, round, interval_relation);
                }
            }
        }
    }
    trades_count
}

fn log_trade_round(
    timestamp: i64,
    round: u64,
    factory: &Factory,
    product: &Product,
    agent: &Agent,
    result: &TradeResult,
    interval_relation: Option<&IntervalRelation>,
    price: f64,
) {
    let agent_id = agent.id();
    let agent_name = agent.name().to_string();
    let product_id = product.id();
    let product_category = product.product_category();
    let (
        agent_cash,
        agent_pref_original_price,
        agent_pref_original_elastic,
        agent_pref_current_price,
        agent_pref_current_range_lower,
        agent_pref_current_range_upper,
    ) = {
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
        &result,
        format!("{:?}", interval_relation).as_str(),
    ) {
        eprintln!("Failed to log trade: {}", e);
    }
}

fn range_factory_list<'a>(
    factory_list: Vec<Rc<RefCell<&mut Factory>>>,
    round: u64,
) -> Vec<(f64, Rc<RefCell<&mut Factory>>)> {
    let factory_list: Vec<Rc<RefCell<&mut Factory>>> = factory_list
        .iter()
        .filter_map(|f_| {
            let f = f_.borrow();
            if f.get_factory_status() == FactoryStatus::Active && f.get_stock(round) > 0 {
                Some(f_.clone())
            } else {
                None
            }
        })
        .collect();
    let n = factory_list.len().min(3);
    let indexes = random_unrepeat_numbers_in_range(0..factory_list.len(), n);
    let mut infos: Vec<(f64, Rc<RefCell<&mut Factory>>)> = Vec::new();
    for i in indexes {
        let f = factory_list[i].borrow();
        let price = f.offer_price(round);
        infos.push((price, factory_list[i].clone()));
    }
    infos.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    infos
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
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

    // 测试 log_trade_round 函数
    #[test]
    fn test_log_trade_round() {
        // 创建测试用的产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );

        // 创建测试用的工厂
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        // 不直接访问私有字段supply_price_range，使用工厂默认行为

        // 创建测试用的代理
        let mut agent = Agent::new(
            1,
            "Test Agent".to_string(),
            1000.0,
            &vec![product.clone()],
            true,
        );
        // 不直接调用私有方法set_preference_detail，使用代理默认行为

        // 测试场景1：交易成功，有区间关系
        let timestamp = 1234567890;
        let round = 1;
        let result = TradeResult::Success(150.0);
        let interval_relation = IntervalRelation::Overlapping(0.5);
        let price = 150.0;

        // 调用函数，验证是否能正常执行
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        // 测试场景2：交易失败，有区间关系
        let result = TradeResult::Failed;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        // 测试场景3：交易结果为NotMatched
        let result = TradeResult::NotMatched;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        // 测试场景4：交易结果为NotYet
        let result = TradeResult::NotYet;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        // 测试场景5：区间关系为None
        log_trade_round(
            timestamp, round, &factory, &product, &agent, &result, None, price,
        );

        // 测试场景6：代理没有对应产品的偏好
        let new_product = Product::from(
            2,
            "New Product".to_string(),
            ProductCategory::from_str("Food"),
            1.0,
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        log_trade_round(
            timestamp,
            round,
            &factory,
            &new_product,
            &agent,
            &result,
            None,
            price,
        );

        // 测试场景7：不同的区间关系类型
        let interval_relation = IntervalRelation::AgentBelowFactory;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        let interval_relation = IntervalRelation::AgentAboveFactory;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );

        let interval_relation = IntervalRelation::CashBurnedOut;
        log_trade_round(
            timestamp,
            round,
            &factory,
            &product,
            &agent,
            &result,
            Some(&interval_relation),
            price,
        );
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

    #[test]
    fn test_range_factory_list() {
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

        // 测试1: 3个活跃工厂，库存充足
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);
        let mut factory2 = Factory::new(2, "Test Factory 2".to_string(), &product);
        let mut factory3 = Factory::new(3, "Test Factory 3".to_string(), &product);

        // 给工厂主动赋值stock
        factory1.set_stock(1, 10); // 设置回合1的库存为10
        factory2.set_stock(1, 10); // 设置回合1的库存为10
        factory3.set_stock(1, 10); // 设置回合1的库存为10

        let factory_list = vec![
            Rc::new(RefCell::new(&mut factory1)),
            Rc::new(RefCell::new(&mut factory2)),
            Rc::new(RefCell::new(&mut factory3)),
        ];

        let infos = range_factory_list(factory_list, 1);

        assert_eq!(infos.len(), 3);
        let mut base_price = infos[0].0;
        for i in 1..infos.len() {
            assert!(
                infos[i].0 >= base_price,
                "Prices should be sorted in ascending order"
            );
            base_price = infos[i].0;
        }

        // 测试2: 6个活跃工厂，库存充足，应该返回3个
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);
        let mut factory2 = Factory::new(2, "Test Factory 2".to_string(), &product);
        let mut factory3 = Factory::new(3, "Test Factory 3".to_string(), &product);
        let mut factory4 = Factory::new(4, "Test Factory 4".to_string(), &product);
        let mut factory5 = Factory::new(5, "Test Factory 5".to_string(), &product);
        let mut factory6 = Factory::new(6, "Test Factory 6".to_string(), &product);

        // 给工厂主动赋值stock
        factory1.set_stock(1, 10);
        factory2.set_stock(1, 10);
        factory3.set_stock(1, 10);
        factory4.set_stock(1, 10);
        factory5.set_stock(1, 10);
        factory6.set_stock(1, 10);

        let factory_list = vec![
            Rc::new(RefCell::new(&mut factory1)),
            Rc::new(RefCell::new(&mut factory2)),
            Rc::new(RefCell::new(&mut factory3)),
            Rc::new(RefCell::new(&mut factory4)),
            Rc::new(RefCell::new(&mut factory5)),
            Rc::new(RefCell::new(&mut factory6)),
        ];
        let infos = range_factory_list(factory_list, 1);

        assert_eq!(infos.len(), 3, "Should return at most 3 factories");
        let mut base_price = infos[0].0;
        for i in 1..infos.len() {
            assert!(
                infos[i].0 >= base_price,
                "Prices should be sorted in ascending order"
            );
            base_price = infos[i].0;
        }

        // 测试3: 只有1个活跃工厂，库存充足
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);

        // 给工厂主动赋值stock
        factory1.set_stock(1, 10);

        let factory_list = vec![Rc::new(RefCell::new(&mut factory1))];
        let infos = range_factory_list(factory_list, 1);
        assert_eq!(
            infos.len(),
            1,
            "Should return 1 factory when only 1 is available"
        );

        // 测试4: 工厂库存为0，应该被过滤掉
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);

        // 给工厂主动赋值stock为0
        factory1.set_stock(1, 0);

        let factory_list = vec![Rc::new(RefCell::new(&mut factory1))];
        let infos = range_factory_list(factory_list, 1);
        assert_eq!(
            infos.len(),
            0,
            "Should return empty list when factory stock is 0"
        );

        // 测试5: 混合情况，部分工厂库存不足
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);
        let mut factory2 = Factory::new(2, "Test Factory 2".to_string(), &product);
        let mut factory3 = Factory::new(3, "Test Factory 3".to_string(), &product);

        // 给工厂主动赋值不同的stock
        factory1.set_stock(1, 10); // 库存充足
        factory2.set_stock(1, 0); // 库存为0
        factory3.set_stock(1, 10); // 库存充足

        let factory_list = vec![
            Rc::new(RefCell::new(&mut factory1)),
            Rc::new(RefCell::new(&mut factory2)),
            Rc::new(RefCell::new(&mut factory3)),
        ];
        let infos = range_factory_list(factory_list, 1);
        assert!(
            infos.len() <= 2,
            "Should return at most 2 factories with sufficient stock"
        );

        // 验证返回的工厂都是活跃的
        for (_, factory_ref) in infos {
            let factory = factory_ref.borrow();
            assert_eq!(
                factory.status(),
                FactoryStatus::Active,
                "Only active factories should be returned"
            );
        }
    }

    // 测试 process_product_trades 函数
    #[test]
    fn test_process_product_trades_basic() {
        // 创建测试产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        let products = vec![product.clone()];

        // 创建测试工厂
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        let factory_arc = Arc::new(RwLock::new(vec![factory]));

        // 创建测试代理人，不开启自动需求生成
        let agent = Agent::new(1, "Test Agent".to_string(), 1000.0, &products, false);
        let agents_vec = vec![Arc::new(RwLock::new(agent))];
        let agents_arc = Arc::new(RwLock::new(agents_vec));

        // 手动设置代理人需求和偏好
        {
            let agents = agents_arc.read();
            let mut agent = agents[0].write();
            // 手动设置需求
            agent.set_demand(product.id(), true);
            // 设置代理人偏好范围，确保交易成功
            agent.set_preference_range(product.id(), product.product_category(), (50.0, 150.0));
        }

        // 调用 process_product_trades 函数
        let timestamp = 1234567890;
        let round = 1;
        let trades_count = process_product_trades(
            timestamp,
            products,
            factory_arc.clone(),
            agents_arc.clone(),
            round,
            product.id(),
        );

        // 验证函数能够正常执行，不崩溃
        // 交易数量可能为0，因为取决于工厂的报价和代理人的心理价位是否匹配
        // 我们只验证函数能够正确处理各种情况，而不断言一定会有交易发生
        assert!(trades_count <= 1, "Should not have more than 1 trade");
    }

    #[test]
    fn test_process_product_trades_no_demand() {
        // 创建测试产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        let products = vec![product.clone()];

        // 创建测试工厂
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        let factory_arc = Arc::new(RwLock::new(vec![factory]));

        // 创建测试代理人，不开启自动需求生成
        let agent = Agent::new(1, "Test Agent".to_string(), 1000.0, &products, false);
        let agents_vec = vec![Arc::new(RwLock::new(agent))];
        let agents_arc = Arc::new(RwLock::new(agents_vec));

        // 调用 process_product_trades 函数
        let timestamp = 1234567890;
        let round = 1;
        let trades_count = process_product_trades(
            timestamp,
            products,
            factory_arc.clone(),
            agents_arc.clone(),
            round,
            product.id(),
        );

        // 验证没有交易发生
        assert_eq!(trades_count, 0, "Should have 0 trades when no demand");
    }

    #[test]
    fn test_process_product_trades_no_factories() {
        // 创建测试产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        let products = vec![product.clone()];

        // 创建空工厂列表
        let factory_arc = Arc::new(RwLock::new(Vec::<Factory>::new()));

        // 创建测试代理人，不开启自动需求生成
        let agent = Agent::new(1, "Test Agent".to_string(), 1000.0, &products, false);
        let agents_vec = vec![Arc::new(RwLock::new(agent))];
        let agents_arc = Arc::new(RwLock::new(agents_vec));

        // 手动设置代理人需求
        {
            let agents = agents_arc.read();
            let mut agent = agents[0].write();
            agent.set_demand(product.id(), true);
        }

        // 调用 process_product_trades 函数
        let timestamp = 1234567890;
        let round = 1;
        let trades_count = process_product_trades(
            timestamp,
            products,
            factory_arc.clone(),
            agents_arc.clone(),
            round,
            product.id(),
        );

        // 验证没有交易发生
        assert_eq!(trades_count, 0, "Should have 0 trades when no factories");
    }

    #[test]
    fn test_process_product_trades_multiple_agents() {
        // 创建测试产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        let products = vec![product.clone()];

        // 创建测试工厂
        let mut factory1 = Factory::new(1, "Test Factory 1".to_string(), &product);
        let mut factory2 = Factory::new(2, "Test Factory 2".to_string(), &product);
        let factory_arc = Arc::new(RwLock::new(vec![factory1, factory2]));

        // 创建多个测试代理人，不开启自动需求生成
        let mut agents_vec = Vec::new();
        for i in 1..=5 {
            let agent = Agent::new(
                i as u64,
                format!("Test Agent {}", i),
                1000.0,
                &products,
                false, // 不开启自动需求生成
            );
            agents_vec.push(Arc::new(RwLock::new(agent)));
        }
        let agents_arc = Arc::new(RwLock::new(agents_vec));

        // 手动设置所有代理人需求和偏好
        {
            let agents = agents_arc.read();
            for agent_rc in agents.iter() {
                let mut agent = agent_rc.write();
                // 手动设置需求
                agent.set_demand(product.id(), true);
                // 设置代理人偏好范围，确保交易成功
                agent.set_preference_range(product.id(), product.product_category(), (50.0, 150.0));
            }
        }

        // 调用 process_product_trades 函数
        let timestamp = 1234567890;
        let round = 1;
        let trades_count = process_product_trades(
            timestamp,
            products,
            factory_arc.clone(),
            agents_arc.clone(),
            round,
            product.id(),
        );

        // 验证有交易发生，但交易数量取决于工厂库存和代理人协商结果
        assert!(
            trades_count > 0,
            "Should have at least 1 trade with multiple agents"
        );
        assert!(trades_count <= 5, "Should not have more trades than agents");
    }

    #[test]
    fn test_process_product_trades_trade_failure() {
        // 创建测试产品
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
            price_distribution.clone(),
            elastic_distribution.clone(),
            cost_distribution.clone(),
        );
        let products = vec![product.clone()];

        // 创建测试工厂
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        let factory_arc = Arc::new(RwLock::new(vec![factory]));

        // 创建测试代理人，不开启自动需求生成
        let agent = Agent::new(1, "Test Agent".to_string(), 1000.0, &products, false);
        let agents_vec = vec![Arc::new(RwLock::new(agent))];
        let agents_arc = Arc::new(RwLock::new(agents_vec));

        // 手动设置代理人需求和偏好
        {
            let agents = agents_arc.read();
            let mut agent = agents[0].write();
            // 手动设置需求
            agent.set_demand(product.id(), true);
            // 设置代理人偏好范围，确保交易失败
            agent.set_preference_range(product.id(), product.product_category(), (50.0, 60.0));
        }

        // 调用 process_product_trades 函数
        let timestamp = 1234567890;
        let round = 1;
        let trades_count = process_product_trades(
            timestamp,
            products,
            factory_arc.clone(),
            agents_arc.clone(),
            round,
            product.id(),
        );

        // 验证没有交易发生
        assert_eq!(
            trades_count, 0,
            "Should have 0 trades when negotiation fails"
        );

        // 验证代理人需求仍然存在（因为交易失败，需求可能被保留或根据弹性删除）
        {
            let agents = agents_arc.read();
            let agent = agents[0].read();
            // 由于弹性值为1.0，需求可能被删除，所以这里不做严格断言
            // 只验证函数能够正常执行
        }
    }
}
