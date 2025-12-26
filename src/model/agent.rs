use crate::logging::LOGGER;
use crate::model::agent::preference::Preference;
use crate::model::factory::Factory;
use crate::model::product::{Product, ProductCategory};
use crate::model::util::{
    gen_new_range_with_price, gen_price_in_range, interval_intersection, round_to_nearest_cent,
    shift_range_by_ratio,
};
use log::debug;
use mysql::prelude::{TextQuery, WithParams};
use parking_lot::RwLock;
use rand::Rng;
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

mod preference;

pub struct Agent {
    id: u64,
    name: String,
    preferences: Arc<RwLock<HashMap<ProductCategory, HashMap<u64, Preference>>>>,
    cash: f64,
    demand: Arc<RwLock<HashMap<u64, bool>>>,
}

/// 区间关系枚举，表示两个区间之间的关系
#[derive(Clone, Debug, PartialEq)]
pub enum IntervalRelation {
    /// 区间重叠，包含重叠范围
    Overlapping(f64),
    /// 代理的价格区间整体低于工厂的价格区间
    AgentBelowFactory,
    /// 代理的价格区间整体高于工厂的价格区间
    AgentAboveFactory,
    /// 代理的现金已耗尽
    CashBurnedOut,
}

/// 交易结果枚举
#[derive(Clone, Debug, PartialEq)]
pub enum TradeResult {
    NotYet,
    /// 未匹配到合适的交易对手
    NotMatched,
    /// 交易成功，包含成交价格
    Success(f64),
    /// 交易失败，未达成交易
    Failed,
}

impl Agent {
    pub fn new(id: u64, name: String, cash: f64, products: &[Product], auto_demand: bool) -> Self {
        // 为每个商品生成preference
        let mut preferences_map: HashMap<ProductCategory, HashMap<u64, Preference>> =
            HashMap::new();
        for product in products {
            let mut preferences = preferences_map
                .entry(product.product_category())
                .or_default();
            preferences.insert(product.id(), Preference::from_product(product));
        }

        let mut agent = Agent {
            id,
            name,
            preferences: Arc::new(RwLock::new(preferences_map)),
            cash,
            demand: Arc::new(RwLock::new(HashMap::new())),
        };
        if auto_demand {
            agent.desire();
        }
        agent
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn preferences(
        &self,
    ) -> parking_lot::RwLockReadGuard<'_, HashMap<ProductCategory, HashMap<u64, Preference>>> {
        self.preferences.read()
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    /// 为agent增加收入，在指定范围内随机生成一个金额
    pub fn income(&mut self, range: (f64, f64)) {
        let mut rng = rand::thread_rng();
        let amount = rng.gen_range(range.0..range.1);
        self.cash += amount;
    }

    pub fn desire(&mut self) {
        let d = self.demand.clone();
        let p = self.preferences.clone();
        let user_id = self.id;
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let categories = vec![
                ProductCategory::Food,
                ProductCategory::Clothing,
                ProductCategory::Transport,
                ProductCategory::Water,
                ProductCategory::Entertainment,
            ];
            loop {
                let wait_time = rng.gen_range(100..500);
                thread::sleep(Duration::from_millis(wait_time));
                let preferences_map = p.read();
                let mut new_demand: Vec<u64> = Vec::new();
                for category in categories.iter() {
                    let preferences = preferences_map.get(category).unwrap();
                    let product_id = insert_demand(preferences, d.clone());
                    if let Some(product_id) = product_id {
                        new_demand.push(product_id);
                    }
                }
                // 更新demand
                let mut demand = d.write();
                for product_id in new_demand.iter() {
                    demand.insert(*product_id, true);
                }
                drop(demand);
            }
        });
    }

    pub fn has_demand(&self, product_id: u64) -> bool {
        let demand = self.demand.read();
        demand.contains_key(&product_id)
    }

    pub fn negotiate(
        &self,
        round: u64,
        product_id: u64,
        product_category: ProductCategory,
        price: f64,
    ) -> (TradeResult, IntervalRelation) {
        if !self.has_demand(product_id) {
            return (TradeResult::NotMatched, IntervalRelation::AgentBelowFactory);
        }
        if self.cash < price {
            return (TradeResult::Failed, IntervalRelation::CashBurnedOut);
        }
        let pg = self.preferences.read();

        // 获取消费者的心理出清区间 (Clearing Range)
        let p = pg
            .get(&product_category)
            .and_then(|cat| cat.get(&product_id))
            .expect("Preference should be initialized");

        let agent_range = p.current_range;
        let (lower, upper) = agent_range;
        let mut interval_relation = IntervalRelation::Overlapping(0.0);
        if price < lower {
            interval_relation = IntervalRelation::AgentAboveFactory;
        } else if price > upper {
            interval_relation = IntervalRelation::AgentBelowFactory;
        } else {
            interval_relation = IntervalRelation::Overlapping(price);
        }
        // 2. 根据区间关系和现金流判断成交结果
        let result = match interval_relation {
            IntervalRelation::Overlapping(actual_price) => TradeResult::Success(actual_price),
            _ => TradeResult::Failed, // 价格不在区间内，谈崩了
        };
        (result, interval_relation)
    }

    pub fn get_specific_preference(
        &self,
        product_id: u64,
        product_category: ProductCategory,
    ) -> Preference {
        let pg = self.preferences.read();
        let p = pg
            .get(&product_category)
            .and_then(|cat| cat.get(&product_id))
            .expect("Preference should be initialized");
        p.clone()
    }

    /// 处理交易失败的逻辑
    /// - `is_agent_below_factory`: 如果为true，表示代理价格低于工厂（商家售价太高），需要上移范围
    /// - 如果为false，表示代理价格高于工厂或余额不足，需要下移范围
    fn handle_trade_failure(
        &mut self,
        factory: &Factory,
        product_id: u64,
        product_category: ProductCategory,
        round: u64,
        interval_relation: IntervalRelation,
        offered_price: Vec<f64>,
    ) {
        // 根据1-preference.elastic的概率决定是否删除demand
        let mut rng = rand::thread_rng();
        let preference = self.get_specific_preference(product_id, product_category);
        // 计算概率：弹性值本身，弹性越大，越容易删除需求
        let delete_probability = preference.original_elastic;
        // 生成随机数（0.0到1.0）
        let random_value = rng.gen_range(0.0..1.0);
        if random_value < delete_probability {
            self.remove_demand(product_id, product_category, round, "remove_by_elasticity");
            return;
        }
        if interval_relation == IntervalRelation::CashBurnedOut {
            return;
        }
        let mut above_count = 0;
        let mut lower_count = 0;
        for price in offered_price.iter() {
            if *price > preference.current_range.1 {
                above_count += 1;
            }
            if *price < preference.current_range.0 {
                lower_count += 1;
            }
        }
        let old_range = preference.current_range;
        let (old_min, old_max) = old_range;
        let mut new_range = preference.current_range;
        let mut min_price = preference.current_price;
        for price in offered_price.iter() {
            min_price = min_price.min(*price);
        }
        if above_count > 0 && lower_count > 0 {
            new_range = gen_new_range_with_price(min_price, preference.current_range, 0.2);
        } else if lower_count > 0 {
            new_range = shift_range_by_ratio(preference.current_range, -0.1);
            new_range = gen_new_range_with_price(min_price, new_range, 0.1);
        } else if above_count > 0 {
            new_range = shift_range_by_ratio(preference.current_range, 0.1);
            new_range = gen_new_range_with_price(min_price, new_range, 0.1);
        } else {
            new_range = preference.current_range;
        }
        self.set_preference_detail(
            product_category,
            product_id,
            Some(min_price),
            Some(new_range),
        );
        let mut logger = LOGGER.write();
        if let Err(e) = logger.log_agent_range_adjustment(
            round, // 使用传入的round参数
            self.id,
            self.name.clone(),
            product_id,
            format!("{:?}", product_category),
            old_range,
            new_range,
            "trade_failed",
            None, // 交易失败，没有价格
        ) {
            eprintln!("Failed to log agent range adjustment: {}", e);
        }
    }

    fn set_preference_detail(
        &mut self,
        product_category: ProductCategory,
        product_id: u64,
        price: Option<f64>,
        range: Option<(f64, f64)>,
    ) {
        if price.is_none() && range.is_none() {
            return;
        }
        let mut preferences_map = self.preferences.write();
        let preferences = preferences_map.get_mut(&product_category).unwrap();
        let preference = preferences.get_mut(&product_id).unwrap();
        if let Some(price) = price {
            preference.current_price = price;
        }
        if let Some(range) = range {
            preference.current_range = range;
        }
    }

    fn handle_trade_success(
        &mut self,
        round: u64,
        product_id: u64,
        product_category: ProductCategory,
        factory: &Factory,
        price: f64,
    ) {
        let mut preferences_map = self.preferences.write();
        let mut preferences = preferences_map.get_mut(&product_category).unwrap();
        let mut preference = preferences.get_mut(&product_id).unwrap();
        let old_range = preference.current_range;
        preference.current_price = price;
        self.cash -= price;
        let old_length = old_range.1 - old_range.0;
        let min_len = (price * 0.05).max(0.1); // 至少保留 5% 的模糊空间
        let new_length = (old_length * 0.9).max(min_len);
        let new_lower = (price - new_length / 2.0).max(0.00);
        let mut new_upper = (price + new_length / 2.0).max(0.00).max(new_lower + 0.1);
        preference.current_range = (new_lower, new_upper);
    }

    fn remove_demand(
        &mut self,
        product_id: u64,
        product_category: ProductCategory,
        round: u64,
        reason: &str,
    ) {
        let mut g = self.demand.write();
        g.remove(&product_id);
        drop(g);

        // 记录需求删除日志
        let preferences_map = self.preferences.read();
        let preferences = preferences_map.get(&product_category).unwrap();
        if let Some(preference) = preferences.get(&product_id) {
            let logger = LOGGER.write();
            if let Err(e) = logger.log_agent_demand_removal(
                round,
                self.id,
                self.name.clone(),
                product_id,
                self.cash,
                Some(preference.original_price),
                Some(preference.original_elastic),
                Some(preference.current_price),
                Some(preference.current_range.0),
                Some(preference.current_range.1),
                reason,
            ) {
                println!("Failed to log agent demand removal: {}", e);
            }
        }
    }

    pub fn settling(
        &mut self,
        factory: &Factory,
        round: u64,
        result: TradeResult,
        interval_relation: IntervalRelation,
        offered_prices: Vec<f64>,
    ) -> (TradeResult, Option<IntervalRelation>) {
        // let has_demand = self.has_demand(factory.product_id());
        // if !has_demand {
        //     return (TradeResult::NotMatched, None);
        // }
        // let interval_relation = self.match_factory(factory);
        // let product_id = factory.product_id();
        //
        // match interval_relation {
        //     IntervalRelation::Overlapping(range) => {
        //         let price = gen_price_in_range(range, self.cash);
        //         if price.is_none() {
        //             self.handle_trade_failure(factory, product_id, round, false);
        //             return (TradeResult::Failed, Some(interval_relation));
        //         }
        //         self.remove_demand(
        //             product_id,
        //             factory.product_category(),
        //             round,
        //             "successful_trade",
        //         );
        //         let price = price.unwrap();
        //         self.cash -= price;
        //         let mut preferences_map = self.preferences.write();
        //         let preferences = preferences_map
        //             .get_mut(&factory.product_category())
        //             .unwrap();
        //         let preference = preferences.get_mut(&product_id).unwrap();
        //         preference.current_price = price;
        //         let (new_min, new_max) =
        //             gen_new_range_with_price(price, preference.current_range, 0.9);
        //         let (old_min, old_max) = preference.current_range;
        //         // 计算变化量，如果小于0.01，则不更新
        //         let min_change = (new_min - old_min).abs();
        //         let max_change = (new_max - old_max).abs();
        //
        //         if min_change >= 0.01 || max_change >= 0.01 {
        //             // 计算变化比例（基于原范围长度）
        //             let old_length = old_max - old_min;
        //             let min_change_value = new_min - old_min;
        //             let max_change_value = new_max - old_max;
        //             let min_change_ratio = if old_length > 0.0 {
        //                 min_change_value / old_length
        //             } else {
        //                 0.0
        //             };
        //             let max_change_ratio = if old_length > 0.0 {
        //                 max_change_value / old_length
        //             } else {
        //                 0.0
        //             };
        //             let mut logger = LOGGER.write();
        //             // 调用日志记录函数
        //             if let Err(e) = logger.log_agent_range_adjustment(
        //                 round, // 使用传入的round参数
        //                 self.id(),
        //                 self.name().to_string(),
        //                 product_id,
        //                 format!("{:?}", factory.product_category()),
        //                 (old_min, old_max),
        //                 (new_min, new_max),
        //                 min_change_value,
        //                 max_change_value,
        //                 min_change_ratio,
        //                 max_change_ratio,
        //                 price, // 交易成功，以成交价格为中心
        //                 "trade_success",
        //                 Some(price), // 交易成功，有价格
        //             ) {
        //                 eprintln!("Failed to log agent range adjustment: {}", e);
        //             }
        //
        //             preference.current_range = (new_min, new_max);
        //         }
        //         return (TradeResult::Success(price), Some(interval_relation));
        //     }
        //     IntervalRelation::AgentBelowFactory => {
        //         // 代理价格低于工厂，商家售价太高，上移3%
        //         self.handle_trade_failure(factory, product_id, round, true);
        //         return (TradeResult::Failed, Some(interval_relation));
        //     }
        //     IntervalRelation::AgentAboveFactory => {
        //         // 代理价格高于工厂，商家售价太低，下移3%
        //         self.handle_trade_failure(factory, product_id, round, false);
        //         return (TradeResult::Failed, Some(interval_relation));
        //     }
        // }
        todo!()
    }
}

#[cfg(test)]
impl Agent {
    pub fn set_cash(&mut self, cash: f64) {
        self.cash = cash;
    }
}

fn insert_demand(
    preference: &HashMap<u64, Preference>,
    demand: Arc<RwLock<HashMap<u64, bool>>>,
) -> Option<u64> {
    let mut rng = rand::thread_rng();
    let mut product_ids = preference.keys().collect::<Vec<_>>();
    product_ids.shuffle(&mut rng);
    for product_id in product_ids.iter() {
        let preference = preference.get(product_id).unwrap();
        let is_already_demanded = {
            let demand = demand.read();
            demand.contains_key(&product_id)
        };
        if is_already_demanded {
            continue;
        }
        let random = rng.gen_range(0.01..0.99);
        if random > preference.original_elastic {
            return Some(**product_id);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::normal_distribute::NormalDistribution;

    #[test]
    fn test_new() {
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = Vec::new(); // 空产品列表

        let agent = Agent::new(id, name.clone(), cash, &products, true);

        assert_eq!(agent.id(), id);
        assert_eq!(agent.name(), name);
        assert_eq!(agent.preferences().len(), 0); // 空map
        assert_eq!(agent.cash(), cash);
    }

    #[test]
    fn test_has_demand_with_demand() {
        // 创建一个测试产品
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food, // 添加缺失的product_category参数
            1.0,
            crate::entity::normal_distribute::NormalDistribution::new(
                10.0,
                product_id,
                "price_dist".to_string(),
                2.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                0.5,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                5.0,
                product_id,
                "cost_dist".to_string(),
                1.0,
            ),
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products, true);

        // 添加需求
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }

        // 验证has_demand返回true
        assert!(
            agent.has_demand(product_id),
            "Agent should have demand for product {}",
            product_id
        );
    }

    #[test]
    fn test_has_demand_without_demand() {
        // 创建一个测试产品
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food, // 添加缺失的product_category参数
            1.0,
            crate::entity::normal_distribute::NormalDistribution::new(
                10.0,
                product_id,
                "price_dist".to_string(),
                2.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                0.5,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                5.0,
                product_id,
                "cost_dist".to_string(),
                1.0,
            ),
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products, true);

        // 清除所有需求，确保没有需求
        {
            let mut demand = agent.demand.write();
            demand.clear();
        }

        // 没有添加需求，验证has_demand返回false
        assert!(
            !agent.has_demand(product_id),
            "Agent should not have demand for product {}",
            product_id
        );
    }

    #[test]
    fn test_remove_demand() {
        // 创建一个测试产品
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            1.0,
            crate::entity::normal_distribute::NormalDistribution::new(
                10.0,
                product_id,
                "price_dist".to_string(),
                2.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                0.5,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                5.0,
                product_id,
                "cost_dist".to_string(),
                1.0,
            ),
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products, false);

        // 添加需求
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }

        // 验证需求存在
        assert!(
            agent.has_demand(product_id),
            "Agent should have demand for product {}",
            product_id
        );

        // 调用remove_demand方法，添加必要的参数
        agent.remove_demand(product_id, ProductCategory::Food, 0, "test_removal");

        // 验证需求被移除
        assert!(
            !agent.has_demand(product_id),
            "Agent should not have demand for product {} after remove_demand",
            product_id
        );
    }

    #[test]
    fn test_remove_demand_when_no_demand() {
        // 创建一个测试产品
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            1.0,
            crate::entity::normal_distribute::NormalDistribution::new(
                10.0,
                product_id,
                "price_dist".to_string(),
                2.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                0.5,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                5.0,
                product_id,
                "cost_dist".to_string(),
                1.0,
            ),
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products, false);

        // 确保没有需求
        {
            let mut demand = agent.demand.write();
            demand.clear();
        }

        // 验证初始状态没有需求
        assert!(
            !agent.has_demand(product_id),
            "Agent should not have demand for product {}",
            product_id
        );

        // 调用remove_demand方法，添加必要的参数，验证没有副作用
        agent.remove_demand(product_id, ProductCategory::Food, 0, "test_removal");

        // 再次验证没有需求
        assert!(
            !agent.has_demand(product_id),
            "Agent should still not have demand for product {} after remove_demand",
            product_id
        );
    }

    #[test]
    fn test_income() {
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let initial_cash = 100.0;
        let products = Vec::new(); // 空产品列表
        let mut agent = Agent::new(id, name, initial_cash, &products, true);

        // 定义收入范围
        let income_range = (50.0, 150.0);
        let (min_income, max_income) = income_range;

        // 记录初始cash
        let initial_cash = agent.cash();

        // 调用income方法
        agent.income(income_range);

        // 验证cash确实增加了
        let final_cash = agent.cash();
        assert!(
            final_cash > initial_cash,
            "Cash should increase after income"
        );

        // 验证增加的金额在指定范围内
        let income_amount = final_cash - initial_cash;
        assert!(
            income_amount >= min_income,
            "Income amount should be at least {}",
            min_income
        );
        assert!(
            income_amount <= max_income,
            "Income amount should be at most {}",
            max_income
        );

        // 测试多次调用，确保每次都能正确增加
        let current_cash = agent.cash();
        agent.income(income_range);
        let new_cash = agent.cash();
        assert!(
            new_cash > current_cash,
            "Cash should increase after second income"
        );
    }

    #[test]
    #[should_panic]
    fn test_negotiate() {
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = Vec::new(); // 空产品列表
        let mut agent = Agent::new(id, name, cash, &products, true);

        // 定义测试参数
        let product_id = 1;
        let product_category = ProductCategory::Food;
        let price = 50.0;

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        // 验证结果
        assert_eq!(
            result,
            TradeResult::Failed,
            "Trade should fail when price is too high"
        );
    }

    #[test]
    fn test_negotiate_with_no_demand() {
        let product_id = 1;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )]; // 空产品列表
        let mut agent = Agent::new(id, name, cash, &products, false);
        // 定义测试参数
        let product_id = 1;
        let product_category = ProductCategory::Food;
        let price = 50.0;

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        // 验证结果
        assert_eq!(
            result,
            TradeResult::NotMatched,
            "Trade should fail when price is too high"
        );
    }

    #[test]
    fn test_negotiate_with_demand() {
        let product_id = 1;
        // 定义测试参数
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )]; // 空产品列表
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }

        let price = 50.0;

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        match result {
            TradeResult::Success(p) => {
                assert_eq!(p, price);
            }
            _ => panic!("Trade should match when price is within demand"),
        }
        match interval_relation {
            IntervalRelation::Overlapping(p) => {
                assert_eq!(p, price);
            }
            _ => panic!("Trade should match when price is within demand"),
        }
    }

    #[test]
    fn test_negotiate_price_below_range() {
        let product_id = 1;
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )];
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }

        let price = 5.0; // 价格低于区间下限

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        // 验证结果
        assert_eq!(
            result,
            TradeResult::Failed,
            "Trade should fail when price is below range"
        );
        assert_eq!(
            interval_relation,
            IntervalRelation::AgentAboveFactory,
            "Interval relation should be AgentAboveFactory when price is below range"
        );
    }

    #[test]
    fn test_negotiate_price_above_range() {
        let product_id = 1;
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )];
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }

        let price = 100.0; // 价格高于区间上限

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        // 验证结果
        assert_eq!(
            result,
            TradeResult::Failed,
            "Trade should fail when price is above range"
        );
        assert_eq!(
            interval_relation,
            IntervalRelation::AgentBelowFactory,
            "Interval relation should be AgentBelowFactory when price is above range"
        );
    }

    #[test]
    fn test_negotiate_with_insufficient_cash() {
        let product_id = 1;
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 30.0; // 现金不足
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )];
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0); // 价格在区间内
            inner_map.insert(product_id, preference);
        }

        let price = 50.0; // 价格在区间内，但现金不足

        // 调用negotiate方法
        let (result, interval_relation) = agent.negotiate(0, product_id, product_category, price);

        // 验证结果
        assert_eq!(
            result,
            TradeResult::Failed,
            "Trade should fail when cash is insufficient"
        );
        assert_eq!(
            interval_relation,
            IntervalRelation::CashBurnedOut,
            "Interval relation should be Overlapping when price is within range"
        );
    }

    #[test]
    fn test_get_specific_preference() {
        let product_id = 1;
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )];
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }
        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(preference.original_price, 0.5);
        assert_eq!(preference.original_elastic, 1.0);
        assert_eq!(preference.current_price, 0.0);
        assert_eq!(preference.current_range, (10.0, 90.0));
    }

    #[test]
    fn test_set_preference_detail() {
        let product_id = 1;
        let product_category = ProductCategory::Food;
        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![Product::from(
            product_id,
            "test_product".to_string(),
            ProductCategory::Food,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        )];
        let mut agent = Agent::new(id, name, cash, &products, false);
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }
        agent.set_preference_detail(product_category, product_id, Some(0.6), Some((11.0, 89.0)));
        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(preference.original_price, 0.5);
        assert_eq!(preference.original_elastic, 1.0);
        assert_eq!(preference.current_price, 0.6);
        assert_eq!(preference.current_range, (11.0, 89.0));
    }

    // 辅助函数：创建测试所需的Agent、Factory和Product
    fn setup_test_environment() -> (Agent, Factory, u64, ProductCategory) {
        let product_id = 1;
        let product_category = ProductCategory::Food;

        // 创建测试产品
        let product = Product::from(
            product_id,
            "test_product".to_string(),
            product_category,
            1.0,
            NormalDistribution::new(10.0, product_id, "price_dist".to_string(), 2.0),
            NormalDistribution::new(0.5, product_id, "elastic_dist".to_string(), 0.1),
            NormalDistribution::new(5.0, product_id, "cost_dist".to_string(), 1.0),
        );

        // 创建测试工厂
        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory = Factory::new(factory_id, factory_name, &product);

        // 创建测试Agent
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(agent_id, agent_name, cash, &products, false);

        // 设置Agent的需求和偏好
        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = Preference::new(0.5, 1.0);
            preference.current_range = (10.0, 90.0);
            inner_map.insert(product_id, preference);
        }

        (agent, factory, product_id, product_category)
    }

    #[test]
    fn test_handle_trade_failure_should_remove_demand_when_random_less_than_elasticity() {
        // 测试：当随机数小于弹性值时，应该删除需求
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为1.0，确保随机数总是小于它
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 1.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::AgentBelowFactory,
            vec![100.0], // 高于上限的价格
        );

        // 验证需求被删除
        assert!(
            !agent.has_demand(product_id),
            "Demand should be removed when random value is less than elasticity"
        );
    }

    #[test]
    fn test_handle_trade_failure_should_not_remove_demand_when_random_greater_than_elasticity() {
        // 测试：当随机数大于弹性值时，不应该删除需求
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为0.0，确保随机数总是大于它
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::AgentBelowFactory,
            vec![100.0], // 高于上限的价格
        );

        // 验证需求仍然存在
        assert!(
            agent.has_demand(product_id),
            "Demand should not be removed when random value is greater than elasticity"
        );
    }

    #[test]
    fn test_handle_trade_failure_should_return_early_when_cash_burned_out() {
        // 测试：当区间关系为CashBurnedOut时，应该提前返回
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 记录初始范围
        let initial_range = {
            let preferences = agent.preferences.read();
            let inner_map = preferences.get(&product_category).unwrap();
            let preference = inner_map.get(&product_id).unwrap();
            preference.current_range.clone()
        };

        // 设置弹性值为0.0，确保不会删除需求
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::CashBurnedOut,
            vec![50.0], // 任何价格
        );

        // 验证范围没有变化
        let new_range = {
            let preferences = agent.preferences.read();
            let inner_map = preferences.get(&product_category).unwrap();
            let preference = inner_map.get(&product_id).unwrap();
            preference.current_range.clone()
        };

        assert_eq!(
            new_range, initial_range,
            "Range should not change when interval relation is CashBurnedOut"
        );
    }

    #[test]
    fn test_handle_trade_failure_with_both_above_and_below_prices() {
        // 测试：当既有高于上限又有低于下限的价格时
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为0.0，确保不会删除需求
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::AgentBelowFactory,
            vec![5.0, 100.0], // 既有低于下限又有高于上限的价格
        );

        // 验证需求仍然存在
        assert!(agent.has_demand(product_id), "Demand should still exist");
    }

    #[test]
    fn test_handle_trade_failure_with_only_below_prices() {
        // 测试：当只有低于下限的价格时
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为0.0，确保不会删除需求
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::AgentAboveFactory,
            vec![5.0, 8.0], // 只有低于下限的价格
        );

        // 验证需求仍然存在
        assert!(agent.has_demand(product_id), "Demand should still exist");
    }

    #[test]
    fn test_handle_trade_failure_with_only_above_prices() {
        // 测试：当只有高于上限的价格时
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为0.0，确保不会删除需求
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::AgentBelowFactory,
            vec![100.0, 120.0], // 只有高于上限的价格
        );

        // 验证需求仍然存在
        assert!(agent.has_demand(product_id), "Demand should still exist");
    }

    #[test]
    fn test_handle_trade_failure_within_range_prices() {
        // 测试：当所有价格都在范围内时
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置弹性值为0.0，确保不会删除需求
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.original_elastic = 0.0;
        }

        // 调用handle_trade_failure方法
        agent.handle_trade_failure(
            &factory,
            product_id,
            product_category,
            0,
            IntervalRelation::Overlapping(50.0),
            vec![50.0, 60.0], // 所有价格都在范围内
        );

        // 验证需求仍然存在
        assert!(agent.has_demand(product_id), "Demand should still exist");
    }

    // handle_trade_success 方法的测试用例
    #[test]
    fn test_handle_trade_success_min_len_0_1() {
        // 测试：当price * 0.05 < 0.1时，min_len取0.1
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 10.0);
        }

        // 使用低价，确保price * 0.05 < 0.1
        let price = 1.0; // 1.0 * 0.05 = 0.05 < 0.1，所以min_len应该取0.1

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        assert_eq!(agent.cash(), 99.0, "Cash should decrease by price");

        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(
            preference.current_price, price,
            "Current price should be updated"
        );
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_min_len_price_percent() {
        // 测试：当price * 0.05 >= 0.1时，min_len取price * 0.05
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 100.0);
        }

        // 使用高价，确保price * 0.05 >= 0.1
        let price = 10.0; // 10.0 * 0.05 = 0.5 >= 0.1，所以min_len应该取0.5

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        assert_eq!(agent.cash(), 90.0, "Cash should decrease by price");

        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(
            preference.current_price, price,
            "Current price should be updated"
        );
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_new_length_min_len() {
        // 测试：当old_length * 0.9 < min_len时，new_length取min_len
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围，使old_length很小
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (50.0, 50.1); // old_length = 0.1
        }

        let price = 50.0;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        assert_eq!(agent.cash(), 50.0, "Cash should decrease by price");

        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(
            preference.current_price, price,
            "Current price should be updated"
        );
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_new_length_old_length_percent() {
        // 测试：当old_length * 0.9 >= min_len时，new_length取old_length * 0.9
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围，使old_length很大
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 100.0); // old_length = 100.0
        }

        let price = 50.0;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        assert_eq!(agent.cash(), 50.0, "Cash should decrease by price");

        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(
            preference.current_price, price,
            "Current price should be updated"
        );
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_new_lower_0_00() {
        // 测试：当price - new_length / 2.0 < 0.00时，new_lower取0.00
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 10.0);
        }

        // 使用低价，确保price - new_length / 2.0 < 0.00
        let price = 0.1;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(preference.current_range.0, 0.0, "new_lower should be 0.00");
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_new_lower_calculated() {
        // 测试：当price - new_length / 2.0 >= 0.00时，new_lower取计算值
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 100.0);
        }

        // 使用高价，确保price - new_length / 2.0 >= 0.00
        let price = 50.0;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        let preference = agent.get_specific_preference(product_id, product_category);
        assert!(
            preference.current_range.0 > 0.0,
            "new_lower should be calculated value"
        );
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
    }

    #[test]
    fn test_handle_trade_success_new_upper_max_with_new_lower_plus_0_1() {
        // 测试：new_upper取new_lower + 0.1的情况
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 0.1);
        }

        // 使用低价
        let price = 0.0;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        let preference = agent.get_specific_preference(product_id, product_category);
        assert_eq!(preference.current_range.0, 0.0, "new_lower should be 0.00");
        assert_eq!(
            preference.current_range.1, 0.1,
            "new_upper should be new_lower + 0.1"
        );
    }

    #[test]
    fn test_handle_trade_success_new_upper_max_price_plus() {
        // 测试：new_upper取price + new_length / 2.0的情况
        let (mut agent, factory, product_id, product_category) = setup_test_environment();

        // 设置初始范围
        {
            let mut preferences = agent.preferences.write();
            let mut inner_map = preferences.entry(product_category).or_default();
            let mut preference = inner_map.get_mut(&product_id).unwrap();
            preference.current_range = (0.0, 100.0);
        }

        // 使用高价
        let price = 50.0;

        // 调用handle_trade_success方法
        agent.handle_trade_success(0, product_id, product_category, &factory, price);

        // 验证结果
        let preference = agent.get_specific_preference(product_id, product_category);
        assert!(
            preference.current_range.1 > preference.current_range.0,
            "Range should be valid"
        );
        assert!(
            preference.current_range.1 > price,
            "new_upper should be greater than price"
        );
    }
}
