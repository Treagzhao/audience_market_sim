use crate::model::agent::preference::Preference;
use crate::model::factory::Factory;
use crate::model::product::Product;
use crate::model::util::interval_intersection;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

mod preference;

pub struct Agent {
    id: u64,
    name: String,
    preferences: Arc<RwLock<HashMap<u64, Preference>>>,
    cash: f64,
    demand: Arc<RwLock<HashMap<u64, bool>>>,
}

/// 交易结果枚举
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
    pub fn new(id: u64, name: String, cash: f64, products: &[Product]) -> Self {
        // 为每个商品生成preference
        let mut preferences_map = HashMap::new();
        for product in products {
            let preference = Preference::from_product(product);
            preferences_map.insert(product.id(), preference);
        }

        let mut agent = Agent {
            id,
            name,
            preferences: Arc::new(RwLock::new(preferences_map)),
            cash,
            demand: Arc::new(RwLock::new(HashMap::new())),
        };
        agent.desire();
        agent
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn preferences(&self) -> std::sync::RwLockReadGuard<'_, HashMap<u64, Preference>> {
        self.preferences.read().unwrap()
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    pub fn desire(&mut self) {
        let d = self.demand.clone();
        let p = self.preferences.clone();
        let user_id = self.id;
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            loop {
                // 从preferences中随机选择一个商品ID
                let product_id = {
                    let preferences = p.read().unwrap();
                    if preferences.is_empty() {
                        continue; // 如果preferences为空，跳过本次循环
                    }

                    // 随机选择一个商品ID
                    let index = rng.gen_range(0..preferences.len());
                    *preferences.keys().nth(index).unwrap()
                };

                // 检查该商品是否已经在demand中
                let is_already_demanded = {
                    let demand = d.read().unwrap();
                    demand.contains_key(&product_id)
                };

                // 如果不在demand中，才添加
                if !is_already_demanded {
                    let mut demand = d.write().unwrap();
                    demand.insert(product_id, true);
                }
                // 随机等待0~500ms
                let wait_time = rng.gen_range(0..500);
                thread::sleep(Duration::from_millis(wait_time));
            }
        });
    }

    pub fn has_demand(&self, product_id: u64) -> bool {
        let demand = self.demand.read().unwrap();
        demand.contains_key(&product_id)
    }

    fn match_factory(&self, factory: &Factory) -> Option<(f64, f64)> {
        let product_id = factory.product_id();
        let g = self.demand.read().unwrap();
        if !g.contains_key(&product_id) {
            return None;
        }
        let pg = self.preferences.read().unwrap();
        let p = pg.get(&product_id).unwrap();
        interval_intersection(p.current_range, factory.supply_price_range())
    }

    /// 处理交易失败的逻辑
    fn handle_trade_failure(&mut self, factory: &Factory, product_id: u64) {
        // 根据1-preference.elastic的概率决定是否删除demand
        let mut rng = rand::thread_rng();

        let mut g = self.preferences.write().unwrap();
        if let Some(preference) = g.get_mut(&product_id) {
            // 计算概率：弹性值本身，弹性越大，越容易删除需求
            let delete_probability = preference.original_elastic;

            // 生成随机数（0.0到1.0）
            let random_value = rng.gen_range(0.0..1.0);

            if random_value < delete_probability {
                // 删除demand
                if let Ok(mut demand) = self.demand.write() {
                    demand.remove(&product_id);
                }
            } else {
                // 不删除demand，更新range
                // 以factory的supply_price_range的lower值为中心
                let center = factory.supply_price_range().0;

                // 当前range长度
                let (old_min, old_max) = preference.current_range;
                let old_length = old_max - old_min;
                let new_length = old_length * 1.1; // 范围增大10%
                let half_new_length = new_length / 2.0;

                // 计算四舍五入后的half_new_length，确保最小单位是0.01
                let round_to_nearest_cent = |x: f64| (x * 100.0).round() / 100.0;
                let rounded_half_length = round_to_nearest_cent(half_new_length);

                // 计算新的range，以center为中心，确保不小于0
                let new_min = round_to_nearest_cent(center - rounded_half_length).max(0.0);
                let mut new_max = round_to_nearest_cent(center + rounded_half_length);

                // 确保max大于min
                if new_max <= new_min {
                    new_max = new_min + 0.01;
                }

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

                    // 打印修改日志
                    println!(
                        "Agent {} ({}) product {} range adjusted (trade failed): [{}, {}] -> [{}, {}] | Change: min={:.4} ({:.2}%), max={:.4} ({:.2}%), center={:.2}",
                        self.id,
                        self.name,
                        product_id,
                        old_min,
                        old_max,
                        new_min,
                        new_max,
                        min_change_value,
                        min_change_ratio * 100.0,
                        max_change_value,
                        max_change_ratio * 100.0,
                        center
                    );

                    preference.current_range = (new_min, new_max);
                }
            }
        }
    }

    pub fn trade(&mut self, factory: &Factory) -> TradeResult {
        let g = self.preferences.write().unwrap();
        if !g.contains_key(&factory.product_id()) {
            return TradeResult::NotMatched;
        }
        drop(g);
        let merge_range = self.match_factory(factory);

        let product_id = factory.product_id();
        if let Some(range) = merge_range {
            // 根据range生成一个随机price值
            let (mut min_price, mut max_price) = range;
            // 确保范围有效，避免min_price等于max_price
            if min_price >= max_price {
                max_price = min_price + 0.01;
            }
            let mut rng = rand::thread_rng();
            let mut price = rng.gen_range(min_price..max_price);

            // 检查price是否大于cash
            if price > self.cash {
                // 如果cash在交集范围内，就用cash作为price
                if self.cash >= min_price && self.cash <= max_price {
                    price = self.cash;
                } else {
                    // cash不在交集范围内，处理交易失败
                    self.handle_trade_failure(factory, product_id);
                    return TradeResult::Failed;
                }
            }

            // 四舍五入price到0.01
            let round_to_nearest_cent = |x: f64| (x * 100.0).round() / 100.0;
            let rounded_price = round_to_nearest_cent(price);

            // 如果价格低于0.01，认为是0.0，不能成交
            if rounded_price < 0.01 {
                self.handle_trade_failure(factory, product_id);
                return TradeResult::Failed;
            }

            let mut g = self.preferences.write().unwrap();
            let mut demand = self.demand.write().unwrap();
            demand.remove(&product_id);
            self.cash -= rounded_price;
            let preference = g.get_mut(&product_id).unwrap();
            preference.current_price = rounded_price;

            // 更新current_range，以新price为中点，范围比之前小10%
            let (old_min, old_max) = preference.current_range;
            let old_length = old_max - old_min;
            let new_length = old_length * 0.9; // 范围缩小10%
            let half_new_length = new_length / 2.0;

            // 计算半长并四舍五入到0.01
            let rounded_half_length = round_to_nearest_cent(half_new_length);

            // 从中点向两边扩展，确保以rounded_price为中心
            let new_min = round_to_nearest_cent(rounded_price - rounded_half_length).max(0.0);
            let mut new_max = round_to_nearest_cent(rounded_price + rounded_half_length);

            // 确保max大于min
            if new_max <= new_min {
                new_max = new_min + 0.01;
            }

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

                // 打印修改日志
                println!(
                    "Agent {} ({}) product {} range adjusted: [{}, {}] -> [{}, {}] | Change: min={:.4} ({:.2}%), max={:.4} ({:.2}%), price={:.2}",
                    self.id(),
                    self.name(),
                    product_id,
                    old_min,
                    old_max,
                    new_min,
                    new_max,
                    min_change_value,
                    min_change_ratio * 100.0,
                    max_change_value,
                    max_change_ratio * 100.0,
                    rounded_price
                );

                preference.current_range = (new_min, new_max);
            }
            return TradeResult::Success(rounded_price);
        } else {
            // 没有交集，处理交易失败
            self.handle_trade_failure(factory, product_id);
            return TradeResult::Failed;
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

        let agent = Agent::new(id, name.clone(), cash, &products);

        assert_eq!(agent.id(), id);
        assert_eq!(agent.name(), name);
        assert_eq!(agent.preferences().len(), 0); // 空map
        assert_eq!(agent.cash(), cash);
    }

    #[test]
    fn test_desire() {
        let id = 2;
        let name = "test_agent_desire".to_string();
        let cash = 100.0;

        // 创建测试产品列表
        let products = vec![
            Product::from(
                1,
                "product_1".to_string(),
                crate::entity::normal_distribute::NormalDistribution::new(
                    10.0,
                    1,
                    "price_dist_1".to_string(),
                    2.0,
                ),
                crate::entity::normal_distribute::NormalDistribution::new(
                    0.5,
                    1,
                    "elastic_dist_1".to_string(),
                    0.1,
                ),
            ),
            Product::from(
                2,
                "product_2".to_string(),
                crate::entity::normal_distribute::NormalDistribution::new(
                    20.0,
                    2,
                    "price_dist_2".to_string(),
                    3.0,
                ),
                crate::entity::normal_distribute::NormalDistribution::new(
                    0.3,
                    2,
                    "elastic_dist_2".to_string(),
                    0.1,
                ),
            ),
            Product::from(
                3,
                "product_3".to_string(),
                crate::entity::normal_distribute::NormalDistribution::new(
                    30.0,
                    3,
                    "price_dist_3".to_string(),
                    4.0,
                ),
                crate::entity::normal_distribute::NormalDistribution::new(
                    0.7,
                    3,
                    "elastic_dist_3".to_string(),
                    0.1,
                ),
            ),
        ];

        let mut agent = Agent::new(id, name, cash, &products);

        // 调用desire方法启动需求生成线程
        agent.desire();

        // 等待1秒，让后台线程有机会生成一些需求
        thread::sleep(Duration::from_secs(1));

        // 检查demand映射是否有内容
        let demand = agent.demand.read().unwrap();
        assert!(
            demand.len() > 0,
            "Demand map should not be empty after calling desire"
        );

        // 获取preferences中的商品ID集合
        let valid_product_ids: std::collections::HashSet<u64> = {
            let preferences = agent.preferences.read().unwrap();
            preferences.keys().cloned().collect()
        };

        // 验证所有需求值都是true，且商品ID在preferences中存在
        for (product_id, is_demanded) in demand.iter() {
            assert!(
                *is_demanded,
                "Demand value for product {product_id} should be true"
            );
            assert!(
                valid_product_ids.contains(product_id),
                "Product ID {product_id} should be in preferences"
            );
        }
    }

    #[test]
    fn test_trade() {
        // 创建Product
        let product_id = 1;
        let product = crate::model::product::Product::from(
            product_id,
            "test_product".to_string(),
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
        );

        // 创建Agent和Product列表
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 100.0;
        let products = vec![product.clone()];
        let mut agent = Agent::new(agent_id, agent_name, initial_cash, &products);

        // 设置Agent的偏好和需求
        let initial_range = (40.0, 60.0);

        {
            let mut preferences = agent.preferences.write().unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            preference.current_range = initial_range;
        }

        {
            let mut demand = agent.demand.write().unwrap();
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
            let preferences = agent.preferences.read().unwrap();
            preferences.get(&product_id).unwrap().current_range
        };

        // 计算交集，确保有交集
        let (agent_min, agent_max) = agent_range;
        let (factory_min, factory_max) = factory_range;
        let overlap_min = agent_min.max(factory_min);
        let overlap_max = agent_max.min(factory_max);

        // 如果没有交集，调整Agent的range，确保有交集
        if overlap_max <= overlap_min {
            let mut preferences = agent.preferences.write().unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            // 设置一个与factory_range有交集的range
            preference.current_range = (factory_min, factory_max + 10.0);
        }

        // 记录交易前的状态
        let initial_cash = agent.cash();

        // 获取初始范围
        let initial_range = {
            let preferences_before = agent.preferences.read().unwrap();
            let preference_before = preferences_before.get(&product_id).unwrap();
            preference_before.current_range
        };

        // 执行交易
        let result = agent.trade(&factory);

        // 验证交易成功
        match result {
            TradeResult::Success(_price) => {
                // 验证需求被移除
                {
                    let demand = agent.demand.read().unwrap();
                    assert!(
                        !demand.contains_key(&product_id),
                        "Demand should be removed after successful trade"
                    );
                }

                // 验证current_price和current_range更新
                let preferences = agent.preferences.read().unwrap();
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
                    "Range should be reduced by 10% (expected: {}, actual: {})",
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
            );

            // 每次尝试创建新的Agent和Factory
            let products = vec![product.clone()];
            let mut agent = Agent::new(agent_id, agent_name.clone(), initial_cash, &products);

            // 设置Agent的偏好和需求，range为(40.0, 100.0)，确保cash(50.0)在范围内
            let initial_range = (40.0, 100.0);

            {
                let mut preferences = agent.preferences.write().unwrap();
                let preference = preferences.get_mut(&product_id).unwrap();
                preference.current_range = initial_range;
            }

            {
                let mut demand = agent.demand.write().unwrap();
                demand.insert(product_id, true);
            }

            let factory_id = 1;
            let factory_name = "test_factory".to_string();
            let factory =
                crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);

            // 执行交易
            let result = agent.trade(&factory);

            // 检查是否交易成功且cash变为0
            if let TradeResult::Success(_price) = result {
                if agent.cash() == 0.0 {
                    // 验证需求被移除
                    let demand = agent.demand.read().unwrap();
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
        );

        // 创建Agent和Product列表
        let products = vec![product.clone()];
        let mut agent = Agent::new(agent_id, agent_name, initial_cash, &products);

        // 设置Agent的偏好和需求，range为(40.0, 60.0)，cash(20.0)不在范围内
        let initial_range = (40.0, 60.0);

        {
            let mut preferences = agent.preferences.write().unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            preference.current_range = initial_range;
        }

        {
            let mut demand = agent.demand.write().unwrap();
            demand.insert(product_id, true);
        }

        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory =
            crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);

        // 记录交易前的状态
        let initial_cash = agent.cash();
        let initial_range = {
            let preferences_before = agent.preferences.read().unwrap();
            preferences_before.get(&product_id).unwrap().current_range
        };

        // 执行交易
        let result = agent.trade(&factory);

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
        let preferences = agent.preferences.read().unwrap();
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
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products);

        // 添加需求
        {
            let mut demand = agent.demand.write().unwrap();
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
        );

        // 创建一个测试agent
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;
        let products = vec![product];
        let mut agent = Agent::new(id, name, cash, &products);

        // 清除所有需求，确保没有需求
        {
            let mut demand = agent.demand.write().unwrap();
            demand.clear();
        }

        // 没有添加需求，验证has_demand返回false
        assert!(
            !agent.has_demand(product_id),
            "Agent should not have demand for product {}",
            product_id
        );
    }
}
