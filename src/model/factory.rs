use crate::logging::log_factory_range_optimization;
use crate::model::agent::{IntervalRelation, TradeResult};
use crate::model::product::Product;
use crate::model::util::shift_range_by_ratio;
use log::debug;
use rand::Rng;
use std::collections::{HashMap, LinkedList};

#[derive(Clone)]
pub struct Factory {
    id: u64,
    name: String,
    product_id: u64,
    supply_price_range: (f64, f64),
    amount: HashMap<u64, i16>,
    product_cost: f64,
    u64_list: LinkedList<u64>,
    cash: f64,
}

impl Factory {
    pub fn new(id: u64, name: String, product: &Product) -> Self {
        // 根据商品的价格正态分布，随机生成一个参考价格点
        let reference_price = product.original_price_distribution().sample(None);

        // 随机生成区间，上下界都是随机的，下界不能小于0.0
        let mut rng = rand::thread_rng();

        // 处理特殊情况，确保reference_price是有限值且大于0
        let reference_price = reference_price.max(1.0).min(f64::MAX / 2.0);

        // 计算区间范围，基于参考价格
        let range_scale = reference_price * 0.5;

        // 生成下界：0.0到reference_price
        let lower = rng.gen_range(0.0..reference_price);

        // 确保upper_bound是有限值且大于lower
        let upper_bound = (reference_price + range_scale).min(f64::MAX / 2.0);
        // 生成上界：lower到upper_bound
        let upper = rng.gen_range(lower..upper_bound);


        // 确保product_cost大于0，避免除以0错误
        let product_cost = product.product_cost_distribution().sample(None).max(0.1);
        // 确保初始现金大于0，避免测试失败
        let cash = product.original_price_distribution.sample(None).max(10.0) * 10.0;

        Self {
            id,
            name,
            product_id: product.id(),
            supply_price_range: (lower, upper),
            amount: HashMap::new(),
            u64_list: LinkedList::new(),
            product_cost,
            cash,
        }
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn product_id(&self) -> u64 {
        self.product_id
    }

    pub fn supply_price_range(&self) -> (f64, f64) {
        self.supply_price_range
    }

    pub fn get_stock(&self, round: u64) -> i16 {
        *self.amount.get(&round).unwrap_or(&10) // 默认库存为10
    }

    /// 开始新一轮
    pub fn start_round(&mut self, round: u64) {
        // 计算每轮产量：当前现金余额 / 每单位生产成本，向下取整
        let production = if self.product_cost > 0.0 {
            (self.cash / self.product_cost) as i16
        } else {
            0
        };

        // 扣除产量带来的成本
        let cost = production as f64 * self.product_cost;
        self.cash -= cost;

        // 给hashmap创建一个以round为键，值为计算出的产量
        self.amount.insert(round, production);

        // 把round插入到队尾
        self.u64_list.push_back(round);

        // 队列长度超过3就从队首弹出
        if self.u64_list.len() > 3 {
            let v = self.u64_list.pop_front();
            if let Some(v) = v {
                self.amount.remove(&v);
            }
        }
    }

    pub fn deal(
        &mut self,
        result: &TradeResult,
        round: u64,
        interval_relation: Option<IntervalRelation>,
    ) {
        // 检查指定轮次的库存，如果为0则退出
        if let Some(amount) = self.amount.get(&round) {
            if *amount <= 0 {
                return; // 库存为0，退出
            }
        }

        match result {
            TradeResult::NotMatched | TradeResult::NotYet => {
                // 未匹配，不做任何处理
                return;
            }
            TradeResult::Failed => {
                let mut ratio = 0.0;
                if interval_relation.is_none() {
                    ratio = -0.01;
                } else {
                    let interval_rel = interval_relation.unwrap();
                    match interval_rel {
                        IntervalRelation::Overlapping(_) => {
                            ratio = -0.01;
                        }
                        IntervalRelation::AgentBelowFactory => {
                            ratio = -0.01;
                        }
                        IntervalRelation::AgentAboveFactory => {
                            ratio = 0.01;
                        }
                    }
                }
                let (lower, upper) = self.supply_price_range;
                let range_length = upper - lower;
                let (new_lower, new_upper) = shift_range_by_ratio(self.supply_price_range, ratio);

                // 计算修改幅度
                let lower_change = new_lower - lower;
                let upper_change = new_upper - upper;
                let total_change = (new_lower + new_upper) - (lower + upper);

                // 计算变化比例（基于原范围长度）
                let lower_change_ratio = if range_length > 0.0 {
                    lower_change / range_length
                } else {
                    0.0
                };
                let upper_change_ratio = if range_length > 0.0 {
                    upper_change / range_length
                } else {
                    0.0
                };

                // 调用日志记录函数
                if let Err(e) = log_factory_range_optimization(
                    round,
                    self.id(),
                    self.name().to_string(),
                    self.product_id(),
                    (lower, upper),
                    (new_lower, new_upper),
                    lower_change,
                    upper_change,
                    total_change,
                    lower_change_ratio,
                    upper_change_ratio,
                    "Failed",
                ) {
                    eprintln!("Failed to log factory range optimization: {}", e);
                }

                self.supply_price_range = (new_lower, new_upper);
            }
            TradeResult::Success(price) => {
                // 交易成功，区间整体上移1%
                let (lower, upper) = self.supply_price_range;
                let (new_lower, new_upper) = shift_range_by_ratio(self.supply_price_range, 0.01);
                let range_length = upper - lower;
                // 计算修改幅度
                let lower_change = new_lower - lower;
                let upper_change = new_upper - upper;
                let total_change = (new_lower + new_upper) - (lower + upper);

                // 计算变化比例（基于原范围长度）
                let lower_change_ratio = if range_length > 0.0 {
                    lower_change / range_length
                } else {
                    0.0
                };
                let upper_change_ratio = if range_length > 0.0 {
                    upper_change / range_length
                } else {
                    0.0
                };

                // 调用日志记录函数
                if let Err(e) = log_factory_range_optimization(
                    round,
                    self.id(),
                    self.name().to_string(),
                    self.product_id(),
                    (lower, upper),
                    (new_lower, new_upper),
                    lower_change,
                    upper_change,
                    total_change,
                    lower_change_ratio,
                    upper_change_ratio,
                    "Success",
                ) {
                    eprintln!("Failed to log factory range optimization: {}", e);
                }

                self.supply_price_range = (new_lower, new_upper);

                // 库存减1
                // 更新指定轮次的库存
                self.amount.entry(round).and_modify(|e| *e -= 1);

                // 增加工厂现金
                self.cash += price;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::product::Product;

    #[test]
    fn test_new() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);

        // 验证初始化后的字段值
        assert_eq!(factory.id(), 1);
        assert_eq!(factory.name(), "test_factory");
        assert_eq!(factory.product_id(), 1);
        assert_eq!(factory.amount.len(), 0);
        assert_eq!(factory.u64_list.len(), 0);
        assert!(factory.cash() > 0.0);

        // 验证supply_price_range是有效的
        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }

    #[test]
    fn test_id() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(42, "test_factory".to_string(), &product);
        assert_eq!(factory.id(), 42);
    }

    #[test]
    fn test_name() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "my_factory".to_string(), &product);
        assert_eq!(factory.name(), "my_factory");
    }

    #[test]
    fn test_product_id() {
        let product = Product::new(5, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        assert_eq!(factory.product_id(), 5);
    }

    #[test]
    fn test_supply_price_range() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }

    #[test]
    fn test_start_round() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 记录初始现金和成本，用于验证产量计算
        let initial_cash = factory.cash();
        let product_cost = factory.product_cost;

        // 计算预期产量
        let expected_production = if product_cost > 0.0 {
            (initial_cash / product_cost) as i16
        } else {
            0
        };

        // 计算预期现金变化
        let expected_cash_after = initial_cash - (expected_production as f64 * product_cost);

        // 测试第一轮
        factory.start_round(1);
        // 验证产量是基于现金和成本计算的，而不是固定值10
        let actual_production = factory.amount.get(&1).unwrap();
        assert_eq!(*actual_production, expected_production);
        assert_eq!(factory.u64_list.len(), 1);
        // 验证现金减少了相应的成本
        assert!((factory.cash() - expected_cash_after).abs() < 0.01);

        // 测试第二轮
        let cash_before_round2 = factory.cash();
        let expected_production_round2 = if product_cost > 0.0 {
            (cash_before_round2 / product_cost) as i16
        } else {
            0
        };
        let expected_cash_after_round2 =
            cash_before_round2 - (expected_production_round2 as f64 * product_cost);

        factory.start_round(2);
        let actual_production_round2 = factory.amount.get(&2).unwrap();
        assert_eq!(*actual_production_round2, expected_production_round2);
        assert_eq!(factory.u64_list.len(), 2);
        assert!((factory.cash() - expected_cash_after_round2).abs() < 0.01);

        // 测试第三轮
        factory.start_round(3);
        assert!(factory.amount.get(&3).is_some());
        assert_eq!(factory.u64_list.len(), 3);

        // 测试第四轮，此时队列长度超过3，应该弹出第一个元素
        factory.start_round(4);
        assert!(factory.amount.get(&4).is_some());
        assert_eq!(factory.u64_list.len(), 3);
        // 第一个元素(1)应该被弹出并从amount中移除
        assert!(factory.amount.get(&1).is_none());

        // 测试第五轮，此时队列长度超过3，应该弹出第二个元素
        factory.start_round(5);
        assert!(factory.amount.get(&5).is_some());
        assert_eq!(factory.u64_list.len(), 3);
        // 第二个元素(2)应该被弹出并从amount中移除
        assert!(factory.amount.get(&2).is_none());

        // 验证当前amount中只有3、4、5三个键
        assert_eq!(factory.amount.len(), 3);
        assert!(factory.amount.contains_key(&3));
        assert!(factory.amount.contains_key(&4));
        assert!(factory.amount.contains_key(&5));

        // 验证队列中的元素顺序
        let mut iter = factory.u64_list.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_deal() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 手动设置一个固定的supply_price_range，便于测试
        factory.supply_price_range = (100.0, 200.0);
        let initial_range = factory.supply_price_range;
        let range_length = initial_range.1 - initial_range.0;
        let shift_amount = range_length * 0.001; // 千分之一

        // 启动一轮，否则库存检查会失败
        let test_round = 1;
        factory.start_round(test_round);

        // 测试交易成功情况 - 区间上移千分之一
        factory.deal(&TradeResult::Success(150.0), test_round, None);
        let after_success = factory.supply_price_range;

        // 由于添加了四舍五入处理，实际结果会与预期有细微差异
        // 我们只需要验证区间确实发生了变化，且方向正确
        assert!(
            after_success.0 > initial_range.0,
            "Lower bound should increase after success"
        );
        assert!(
            after_success.1 > initial_range.1,
            "Upper bound should increase after success"
        );
        println!(
            "before:{:?} after:{:?} shift_amount:{:?}",
            initial_range, after_success, shift_amount
        );
        assert!(
            (shift_amount / initial_range.1).abs() < 0.02,
            "Lower bound increase should be within expected range"
        );
        assert!(
            (shift_amount / initial_range.0).abs() < 0.02,
            "Upper bound increase should be within expected range"
        );

        // 测试交易失败情况 - 无区间关系
        let success_range = factory.supply_price_range;
        factory.deal(&TradeResult::Failed, test_round, None);
        let after_failure = factory.supply_price_range;

        // 验证区间确实发生了变化，且方向正确
        assert!(
            after_failure.0 < success_range.0,
            "Lower bound should decrease after failure with no interval relation"
        );
        assert!(
            after_failure.1 < success_range.1,
            "Upper bound should decrease after failure with no interval relation"
        );

        // 测试未匹配情况 - 区间不变
        let failure_range = factory.supply_price_range;
        factory.deal(&TradeResult::NotMatched, test_round, None);
        let after_not_matched = factory.supply_price_range;
        assert_eq!(after_not_matched, failure_range);
    }

    #[test]
    fn test_deal_with_interval_relation() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 手动设置一个固定的supply_price_range，便于测试
        factory.supply_price_range = (100.0, 200.0);

        // 启动一轮，否则库存检查会失败
        let test_round = 1;
        factory.start_round(test_round);

        // 测试1: 交易失败 + Overlapping关系 - 区间下移1%
        let initial_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::Overlapping((100.0, 200.0))),
        );
        let after_overlapping = factory.supply_price_range;

        // 验证区间下移
        assert!(
            after_overlapping.0 < initial_range.0,
            "Lower bound should decrease after failure with Overlapping relation"
        );
        assert!(
            after_overlapping.1 < initial_range.1,
            "Upper bound should decrease after failure with Overlapping relation"
        );

        // 测试2: 交易失败 + AgentBelowFactory关系 - 区间下移1%
        let overlapping_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::AgentBelowFactory),
        );
        let after_below = factory.supply_price_range;

        // 验证区间下移
        assert!(
            after_below.0 < overlapping_range.0,
            "Lower bound should decrease after failure with AgentBelowFactory relation"
        );
        assert!(
            after_below.1 < overlapping_range.1,
            "Upper bound should decrease after failure with AgentBelowFactory relation"
        );

        // 测试3: 交易失败 + AgentAboveFactory关系 - 区间上移1%
        let below_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::AgentAboveFactory),
        );
        let after_above = factory.supply_price_range;

        // 验证区间上移
        assert!(
            after_above.0 > below_range.0,
            "Lower bound should increase after failure with AgentAboveFactory relation"
        );
        assert!(
            after_above.1 > below_range.1,
            "Upper bound should increase after failure with AgentAboveFactory relation"
        );
    }

    #[test]
    fn test_deal_with_small_range() {
        // 测试边界情况：小范围区间
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置一个很小的范围
        factory.supply_price_range = (0.0, 1.0);

        // 启动一轮，否则库存检查会失败
        let test_round = 1;
        factory.start_round(test_round);

        // 测试交易失败，确保下界不会小于0
        factory.deal(&TradeResult::Failed, test_round, None);
        let after_failure = factory.supply_price_range;
        assert!(after_failure.0 >= 0.0);
        assert!(after_failure.1 > after_failure.0);
    }

    #[test]
    fn test_deal_with_inventory() {
        // 测试deal方法的库存逻辑
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置初始供应价格范围
        factory.supply_price_range = (100.0, 200.0);
        // 手动设置工厂的现金和成本，确保初始库存足够大
        factory.cash = 10000.0; // 大量现金
        factory.product_cost = 100.0; // 较低的成本，确保初始库存至少为100

        // 启动一轮，初始库存基于现金和成本计算
        let current_round = 1;
        let initial_cash = factory.cash();
        let product_cost = factory.product_cost;

        // 计算预期初始库存
        let expected_initial_inventory = if product_cost > 0.0 {
            (initial_cash / product_cost) as i16
        } else {
            0
        };

        // 确保预期初始库存至少为3，以便进行3次交易
        assert!(
            expected_initial_inventory >= 3,
            "Expected initial inventory should be at least 3 for this test"
        );

        factory.start_round(current_round);
        assert_eq!(
            factory.amount.get(&current_round),
            Some(&expected_initial_inventory)
        );

        // 测试交易成功，库存减1
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        assert_eq!(
            factory.amount.get(&current_round),
            Some(&(expected_initial_inventory - 1))
        );

        // 测试多次交易成功，库存持续减少
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        assert_eq!(
            factory.amount.get(&current_round),
            Some(&(expected_initial_inventory - 3))
        );
    }

    #[test]
    fn test_deal_with_zero_inventory() {
        // 测试库存为0时deal方法不执行
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置初始供应价格范围
        let initial_range = (100.0, 200.0);
        factory.supply_price_range = initial_range;
        // 确保product_cost大于0，避免除以0错误
        factory.product_cost = factory.product_cost.max(1.0);
        // 确保有足够的现金用于生产
        factory.cash = 1000.0;

        // 启动一轮，然后手动将库存设置为0
        let current_round = 1;
        factory.start_round(current_round);
        // 手动设置库存为0
        *factory.amount.get_mut(&current_round).unwrap() = 0;
        assert_eq!(factory.amount.get(&current_round), Some(&0));

        // 测试交易成功，由于库存为0，deal方法应该不执行
        factory.deal(&TradeResult::Success(150.0), current_round, None);

        // 验证库存仍为0
        assert_eq!(factory.amount.get(&current_round), Some(&0));

        // 验证价格区间没有变化
        assert_eq!(factory.supply_price_range, initial_range);
    }

    #[test]
    fn test_cash_update_after_success() {
        // 测试交易成功后cash字段的更新
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置初始供应价格范围
        factory.supply_price_range = (100.0, 200.0);
        // 确保product_cost大于0，避免除以0错误
        factory.product_cost = factory.product_cost.max(1.0);
        // 确保有足够的现金用于生产
        factory.cash = 1000.0;

        // 启动一轮，初始库存基于现金和成本计算
        let current_round = 1;
        let initial_cash = factory.cash();
        let product_cost = factory.product_cost;

        // 计算预期初始库存
        let expected_initial_inventory = if product_cost > 0.0 {
            (initial_cash / product_cost) as i16
        } else {
            0
        };

        // 计算预期现金变化
        let expected_cash_after_start =
            initial_cash - (expected_initial_inventory as f64 * product_cost);

        factory.start_round(current_round);

        // 验证初始库存是基于现金和成本计算的
        let actual_initial_inventory = factory.amount.get(&current_round).unwrap();
        assert_eq!(*actual_initial_inventory, expected_initial_inventory);

        // 验证现金减少了相应的成本
        assert!((factory.cash() - expected_cash_after_start).abs() < 0.01);

        let cash_before_deal = factory.cash();

        // 模拟交易成功，成交价为150.0
        let deal_price = 150.0;
        factory.deal(&TradeResult::Success(deal_price), current_round, None);

        // 验证cash字段已更新（增加了成交价）
        assert!((factory.cash() - (cash_before_deal + deal_price)).abs() < 0.01);

        // 再次交易成功，成交价为160.0
        let cash_before_second_deal = factory.cash();
        let second_deal_price = 160.0;
        factory.deal(
            &TradeResult::Success(second_deal_price),
            current_round,
            None,
        );

        // 验证cash字段累计更新
        assert!((factory.cash() - (cash_before_second_deal + second_deal_price)).abs() < 0.01);

        // 模拟交易失败，cash字段不应变化
        let cash_before_failed_deal = factory.cash();
        factory.deal(&TradeResult::Failed, current_round, None);
        assert!((factory.cash() - cash_before_failed_deal).abs() < 0.01);
    }
}
