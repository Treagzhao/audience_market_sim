use crate::logging::LOGGER;
use crate::model::agent::preference::Preference;
use crate::model::factory::Factory;
use crate::model::product::{Product, ProductCategory};
use crate::model::util::{
    gen_new_range_with_price, gen_price_in_range, interval_intersection, round_to_nearest_cent,
};
use log::debug;
use mysql::prelude::{TextQuery, WithParams};
use parking_lot::RwLock;
use rand::Rng;
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
#[derive(Clone, Debug)]
pub enum IntervalRelation {
    /// 区间重叠，包含重叠范围
    Overlapping((f64, f64)),
    /// 代理的价格区间整体低于工厂的价格区间
    AgentBelowFactory,
    /// 代理的价格区间整体高于工厂的价格区间
    AgentAboveFactory,
}

/// 交易结果枚举
#[derive(Clone)]
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
            loop {}
        });
    }

    pub fn has_demand(&self, product_id: u64) -> bool {
        let demand = self.demand.read();
        demand.contains_key(&product_id)
    }

    fn match_factory(&self, factory: &Factory) -> IntervalRelation {
        let product_id = factory.product_id();
        let product_category = factory.product_category();

        let pg = self.preferences.read();
        let p = pg.get(&product_category).unwrap().get(&product_id).unwrap();

        let agent_range = p.current_range;
        let factory_range = factory.supply_price_range();

        match interval_intersection(agent_range, factory_range) {
            Some(overlap) => IntervalRelation::Overlapping(overlap),
            None => {
                // 判断区间关系
                let (agent_min, agent_max) = agent_range;
                let (factory_min, factory_max) = factory_range;

                if agent_max < factory_min {
                    // 代理的价格区间整体低于工厂的价格区间
                    IntervalRelation::AgentBelowFactory
                } else {
                    // 代理的价格区间整体高于工厂的价格区间
                    IntervalRelation::AgentAboveFactory
                }
            }
        }
    }

    /// 处理交易失败的逻辑
    /// - `is_agent_below_factory`: 如果为true，表示代理价格低于工厂（商家售价太高），需要上移范围
    /// - 如果为false，表示代理价格高于工厂或余额不足，需要下移范围
    fn handle_trade_failure(
        &mut self,
        factory: &Factory,
        product_id: u64,
        round: u64,
        is_agent_below_factory: bool,
    ) {
        // 根据1-preference.elastic的概率决定是否删除demand
        let mut rng = rand::thread_rng();
        let mut g = self.preferences.write();
        let category = factory.product_category();
        let mut preferences = g.get_mut(&category).unwrap();
        if let Some(preference) = preferences.get_mut(&product_id) {
            // 计算概率：弹性值本身，弹性越大，越容易删除需求
            let delete_probability = preference.original_elastic;
            // 生成随机数（0.0到1.0）
            let random_value = rng.gen_range(0.0..1.0);

            if random_value < delete_probability {
                // 删除demand
                {
                    // 新增作用域括号
                    let mut demand = self.demand.write();
                    demand.remove(&product_id);
                    drop(demand);
                    // 记录需求删除日志
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
                        "elasticity_based_removal",
                    ) {
                        eprintln!("Failed to log agent demand removal: {}", e);
                    }
                } // 关闭作用域括号
            } else {
                // 不删除demand，更新range
                let (old_min, old_max) = preference.current_range;
                let old_length = old_max - old_min;

                // 计算移动的量：当前范围总长度的3%
                let shift_amount = old_length * 0.03;
                // 计算扩大的量：当前范围总长度的1%
                let expand_amount = old_length * 0.01;

                // 四舍五入到最近的0.01
                let round_to_nearest_cent = |x: f64| (x * 100.0).round() / 100.0;
                let rounded_shift = round_to_nearest_cent(shift_amount).max(0.01);
                let rounded_expand = round_to_nearest_cent(expand_amount).max(0.01);

                // 根据情况计算新的范围
                let (mut new_min, mut new_max) = if is_agent_below_factory {
                    // 商家售价太高，代理价格低于工厂，上移3%
                    let shifted_min = round_to_nearest_cent(old_min + rounded_shift);
                    let shifted_max = round_to_nearest_cent(old_max + rounded_shift);
                    (shifted_min, shifted_max)
                } else {
                    // 商家售价太低或余额不足，下移3%
                    let shifted_min = round_to_nearest_cent(old_min - rounded_shift);
                    let shifted_max = round_to_nearest_cent(old_max - rounded_shift);
                    (shifted_min, shifted_max)
                };

                // 扩大范围1%
                new_min = round_to_nearest_cent(new_min - rounded_expand);
                new_max = round_to_nearest_cent(new_max + rounded_expand);

                // 确保最小值不小于0.0
                new_min = new_min.max(0.0);

                // 确保max大于min，且至少有0.01的差距
                let new_max = if new_max <= new_min {
                    new_min + 0.01
                } else {
                    new_max
                };

                // 计算变化量，如果小于0.01，则不更新
                let min_change = (new_min - old_min).abs();
                let max_change = (new_max - old_max).abs();

                if min_change >= 0.01 || max_change >= 0.01 {
                    // 计算变化比例（基于原范围长度）
                    let old_length = old_max - old_min;
                    let min_change_value = new_min - old_min;
                    let max_change_value = new_max - old_max;
                    let min_change_ratio = if old_length > 0.0 {
                        min_change_value / old_length
                    } else {
                        0.0
                    };
                    let max_change_ratio = if old_length > 0.0 {
                        max_change_value / old_length
                    } else {
                        0.0
                    };

                    // 计算新的中心
                    let old_center = (old_min + old_max) / 2.0;
                    let new_center = (new_min + new_max) / 2.0;
                    let rounded_new_center = round_to_nearest_cent(new_center);
                    let mut logger = LOGGER.write();
                    // 调用日志记录函数
                    if let Err(e) = logger.log_agent_range_adjustment(
                        round, // 使用传入的round参数
                        self.id,
                        self.name.clone(),
                        product_id,
                        (old_min, old_max),
                        (new_min, new_max),
                        min_change_value,
                        max_change_value,
                        min_change_ratio,
                        max_change_ratio,
                        rounded_new_center,
                        "trade_failed",
                        None, // 交易失败，没有价格
                    ) {
                        eprintln!("Failed to log agent range adjustment: {}", e);
                    }

                    preference.current_range = (new_min, new_max);
                }
            }
        }
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

    pub fn trade(
        &mut self,
        factory: &Factory,
        round: u64,
    ) -> (TradeResult, Option<IntervalRelation>) {
        let has_demand = self.has_demand(factory.product_id());
        if !has_demand {
            return (TradeResult::NotMatched, None);
        }
        let interval_relation = self.match_factory(factory);
        let product_id = factory.product_id();

        match interval_relation {
            IntervalRelation::Overlapping(range) => {
                let price = gen_price_in_range(range, self.cash);
                if price.is_none() {
                    self.handle_trade_failure(factory, product_id, round, false);
                    return (TradeResult::Failed, Some(interval_relation));
                }
                self.remove_demand(
                    product_id,
                    factory.product_category(),
                    round,
                    "successful_trade",
                );
                let price = price.unwrap();
                self.cash -= price;
                let mut preferences_map = self.preferences.write();
                let preferences = preferences_map
                    .get_mut(&factory.product_category())
                    .unwrap();
                let preference = preferences.get_mut(&product_id).unwrap();
                preference.current_price = price;
                let (new_min, new_max) =
                    gen_new_range_with_price(price, preference.current_range, 0.9);
                let (old_min, old_max) = preference.current_range;
                // 计算变化量，如果小于0.01，则不更新
                let min_change = (new_min - old_min).abs();
                let max_change = (new_max - old_max).abs();

                if min_change >= 0.01 || max_change >= 0.01 {
                    // 计算变化比例（基于原范围长度）
                    let old_length = old_max - old_min;
                    let min_change_value = new_min - old_min;
                    let max_change_value = new_max - old_max;
                    let min_change_ratio = if old_length > 0.0 {
                        min_change_value / old_length
                    } else {
                        0.0
                    };
                    let max_change_ratio = if old_length > 0.0 {
                        max_change_value / old_length
                    } else {
                        0.0
                    };
                    let mut logger = LOGGER.write();
                    // 调用日志记录函数
                    if let Err(e) = logger.log_agent_range_adjustment(
                        round, // 使用传入的round参数
                        self.id(),
                        self.name().to_string(),
                        product_id,
                        (old_min, old_max),
                        (new_min, new_max),
                        min_change_value,
                        max_change_value,
                        min_change_ratio,
                        max_change_ratio,
                        price, // 交易成功，以成交价格为中心
                        "trade_success",
                        Some(price), // 交易成功，有价格
                    ) {
                        eprintln!("Failed to log agent range adjustment: {}", e);
                    }

                    preference.current_range = (new_min, new_max);
                }
                return (TradeResult::Success(price), Some(interval_relation));
            }
            IntervalRelation::AgentBelowFactory => {
                // 代理价格低于工厂，商家售价太高，上移3%
                self.handle_trade_failure(factory, product_id, round, true);
                return (TradeResult::Failed, Some(interval_relation));
            }
            IntervalRelation::AgentAboveFactory => {
                // 代理价格高于工厂，商家售价太低，下移3%
                self.handle_trade_failure(factory, product_id, round, false);
                return (TradeResult::Failed, Some(interval_relation));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_trade() {
        // 创建Product
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            crate::entity::normal_distribute::NormalDistribution::new(
                50.0,
                product_id,
                "price_dist".to_string(),
                5.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                1.0,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                25.0,
                product_id,
                "cost_dist".to_string(),
                3.0,
            ),
        );

        // 创建Agent和Product列表
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 100.0;
        let products = vec![product.clone()];
        let mut agent = Agent::new(agent_id, agent_name, initial_cash, &products, false);

        // 设置Agent的偏好和需求
        let initial_range = (40.0, 60.0);

        {
            let mut preferences_map = agent.preferences.write();
            let preferences = preferences_map
                .get_mut(&product.product_category())
                .unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            preference.current_range = initial_range;
        }

        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }

        // 创建Factory
        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory =
            crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);

        // 确保Factory的supply_price_range与Agent的current_range有交集
        let factory_range = factory.supply_price_range();

        // 获取Agent的range，使用独立的代码块确保借用及时释放
        let agent_range = {
            let preferences_map = agent.preferences.read();
            let preferences = preferences_map.get(&product.product_category()).unwrap();
            preferences.get(&product_id).unwrap().current_range
        };

        // 计算交集，确保有交集
        let (agent_min, agent_max) = agent_range;
        let (factory_min, factory_max) = factory_range;
        let overlap_min = agent_min.max(factory_min);
        let overlap_max = agent_max.min(factory_max);

        // 如果没有交集，调整Agent的range，确保有交集
        if overlap_max <= overlap_min {
            let mut preferences_map = agent.preferences.write();
            let preferences = preferences_map
                .get_mut(&product.product_category())
                .unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            // 设置一个与factory_range有交集的range
            preference.current_range = (factory_min, factory_max + 10.0);
        }

        // 记录交易前的状态
        let initial_cash = agent.cash();

        // 获取初始范围
        let initial_range = {
            let preferences_before_map = agent.preferences.read();
            let preferences_before = preferences_before_map
                .get(&product.product_category())
                .unwrap();
            let preference_before = preferences_before.get(&product_id).unwrap();
            preference_before.current_range
        };

        // 执行交易
        let (result, _interval_relation) = agent.trade(&factory, 0);

        // 验证交易成功
        match result {
            TradeResult::Success(_price) => {
                // 验证需求被移除
                {
                    let demand = agent.demand.read();
                    assert!(
                        !demand.contains_key(&product_id),
                        "Demand should be removed after successful trade"
                    );
                }

                // 验证current_price和current_range更新
                let preferences_map = agent.preferences.read();
                let preferences = preferences_map.get(&product.product_category()).unwrap();
                let preference = preferences.get(&product_id).unwrap();

                // 验证cash减少
                assert!(
                    agent.cash() < initial_cash,
                    "Cash should decrease after trade"
                );

                // 验证current_price被更新
                assert!(
                    preference.current_price > 0.0,
                    "Current price should be updated"
                );

                // 验证current_range以新价格为中点，范围缩小10%
                let (new_min, new_max) = preference.current_range;
                let expected_length = (initial_range.1 - initial_range.0) * 0.9;
                let actual_length = new_max - new_min;
                assert!(
                    (actual_length - expected_length).abs() < expected_length * 0.2,
                    "Range should be reduced by 10% (expected: {}, actual: {})
",
                    expected_length,
                    actual_length
                ); // 允许20%的误差，考虑四舍五入的影响

                // 验证新范围以交易价格为中点
                let midpoint = (new_min + new_max) / 2.0;
                assert!(
                    (midpoint - preference.current_price).abs() < 0.001,
                    "Range should be centered at trade price (midpoint: {}, price: {})
",
                    midpoint,
                    preference.current_price
                );

                // 验证范围不小于0
                assert!(new_min >= 0.0, "Range minimum should be >= 0");
                assert!(new_max > new_min, "Range maximum should be > minimum");
            }
            _ => panic!("Trade should succeed with overlapping ranges"),
        }
    }

    #[test]
    fn test_trade_insufficient_cash_but_in_range() {
        // 测试场景：现金不足但在交集范围内，应该用全部现金交易
        // 由于trade方法内部使用随机数生成价格，我们无法直接控制生成的价格
        // 因此我们需要多次尝试，直到得到我们想要的测试场景

        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 50.0; // 现金不足

        let product_id = 1;

        // 多次尝试，直到交易成功或达到最大尝试次数
        let max_attempts = 100;
        let mut success = false;

        for _ in 0..max_attempts {
            // 创建Product，确保其price_distribution的均值为60.0，标准差为20.0
            let product = crate::model::product::Product::from(
                product_id,
                "test_product".to_string(),
                crate::model::product::ProductCategory::Food,
                crate::entity::normal_distribute::NormalDistribution::new(
                    60.0,
                    product_id,
                    "price_dist".to_string(),
                    20.0,
                ),
                crate::entity::normal_distribute::NormalDistribution::new(
                    1.0,
                    product_id,
                    "elastic_dist".to_string(),
                    0.1,
                ),
                crate::entity::normal_distribute::NormalDistribution::new(
                    30.0,
                    product_id,
                    "cost_dist".to_string(),
                    5.0,
                ),
            );

            // 每次尝试创建新的Agent和Factory
            let products = vec![product.clone()];
            let mut agent = Agent::new(agent_id, agent_name.clone(), initial_cash, &products, true);

            // 设置Agent的偏好和需求，range为(40.0, 100.0)，确保cash(50.0)在范围内
            let initial_range = (40.0, 100.0);

            {
                let mut preferences_map = agent.preferences.write();
                let preferences = preferences_map
                    .get_mut(&product.product_category())
                    .unwrap();
                let preference = preferences.get_mut(&product_id).unwrap();
                preference.current_range = initial_range;
            }

            {
                let mut demand = agent.demand.write();
                demand.insert(product_id, true);
            }

            let factory_id = 1;
            let factory_name = "test_factory".to_string();
            let factory =
                crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);

            // 执行交易
            let (result, _interval_relation) = agent.trade(&factory, 0);

            // 检查是否交易成功且cash变为0
            if let TradeResult::Success(_price) = result {
                if agent.cash() == 0.0 {
                    // 验证需求被移除
                    let demand = agent.demand.read();
                    if !demand.contains_key(&product_id) {
                        success = true;
                        break;
                    }
                }
            }
        }

        // 确保至少有一次成功
        assert!(
            success,
            "Trade should succeed when cash is in range after multiple attempts"
        );
    }

    #[test]
    fn test_trade_insufficient_cash_out_of_range() {
        // 测试场景：现金不足且不在交集范围内，应该走else分支

        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 20.0; // 现金不足，且不在range内

        let product_id = 1;

        // 创建Product，确保其supply_price_range与Agent的range有交集，但cash不在范围内
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            crate::entity::normal_distribute::NormalDistribution::new(
                50.0,
                product_id,
                "price_dist".to_string(),
                5.0,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                1.0,
                product_id,
                "elastic_dist".to_string(),
                0.1,
            ),
            crate::entity::normal_distribute::NormalDistribution::new(
                25.0,
                product_id,
                "cost_dist".to_string(),
                3.0,
            ),
        );

        // 创建Agent和Product列表
        let products = vec![product.clone()];
        let mut agent = Agent::new(agent_id, agent_name, initial_cash, &products, true);

        // 设置Agent的偏好和需求，range为(40.0, 60.0)，cash(20.0)不在范围内
        let initial_range = (40.0, 60.0);

        {
            let mut preferences_map = agent.preferences.write();
            let preferences = preferences_map
                .get_mut(&product.product_category())
                .unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            preference.current_range = initial_range;
        }

        {
            let mut demand = agent.demand.write();
            demand.insert(product_id, true);
        }

        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory =
            crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);

        // 记录交易前的状态
        let initial_cash = agent.cash();
        let initial_range = {
            let preferences_map = agent.preferences.read();
            let preferences = preferences_map.get(&product.product_category()).unwrap();
            preferences.get(&product_id).unwrap().current_range
        };

        // 执行交易
        let (result, _interval_relation) = agent.trade(&factory, 0);

        // 验证交易失败
        match result {
            TradeResult::Failed => {}
            _ => panic!("Trade should fail when cash is out of range"),
        }

        // 验证cash没有变化
        assert_eq!(
            agent.cash(),
            initial_cash,
            "Cash should not change after failed trade"
        );

        // 验证range可能被更新（取决于随机概率）
        let preferences_map = agent.preferences.read();
        let preferences = preferences_map.get(&product.product_category()).unwrap();
        let preference = preferences.get(&product_id).unwrap();
        let new_range = preference.current_range;

        // 由于有随机因素，我们只能验证range可能变化，不能断言一定会变化
        // 但可以验证range的有效性
        assert!(new_range.0 >= 0.0);
        assert!(new_range.1 > new_range.0);
    }

    #[test]
    fn test_has_demand_with_demand() {
        // 创建一个测试产品
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food, // 添加缺失的product_category参数
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
            crate::model::product::ProductCategory::Food, // 添加缺失的product_category参数
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
}
