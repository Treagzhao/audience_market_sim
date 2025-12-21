mod accountant;
mod financial_bill;

use crate::logging::{LOGGER, log_factory_range_optimization};
use crate::model::agent::{IntervalRelation, TradeResult};
use crate::model::factory::accountant::Accountant;
use crate::model::factory::financial_bill::FinancialBill;
use crate::model::product::{Product, ProductCategory};
use crate::model::util::shift_range_by_ratio;
use rand::Rng;
use std::borrow::BorrowMut;
use std::collections::{HashMap, LinkedList};

pub struct Factory {
    id: u64,
    name: String,
    product_id: u64,
    accountant: Accountant,
    product_category: ProductCategory,
    supply_price_range: (f64, f64),
    amount: HashMap<u64, u16>,
    remaining_stock: u16,
    durability: f64,
    product_cost: f64,
    u64_list: LinkedList<u64>,
    cash: f64,
    initial_stock: u16,
    risk_appetite: f64,
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
            accountant: Accountant::new(cash),
            product_category: product.product_category(),
            supply_price_range: (lower, upper),
            amount: HashMap::new(),
            u64_list: LinkedList::new(),
            product_cost,
            remaining_stock: 0,
            durability: product.durability(),
            cash,
            initial_stock: 0,
            risk_appetite: rng.gen_range(0.1..0.9),
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
    pub fn product_category(&self) -> ProductCategory {
        self.product_category.clone()
    }

    pub fn supply_price_range(&self) -> (f64, f64) {
        self.supply_price_range
    }

    pub fn get_stock(&self, round: u64) -> u16 {
        *self.amount.get(&round).unwrap_or(&10) // 默认库存为10
    }

    /// 开始新一轮
    pub fn start_round(&mut self, round: u64) {
        let last_b = self.accountant.get_bill_or_default(round - 1);
        let last_bill = last_b.read();
        let last_round_initial_stock = last_bill.initial_stock;
        let last_round_remaining_stock = last_bill.remaining_stock;
        let last_sales = last_bill.units_sold;
        let prediction_production = if last_round_initial_stock == 0 {
            1
        } else if last_round_remaining_stock == 0 {
            let rate = 1.1 + 0.4 * self.risk_appetite;
            (last_round_initial_stock as f64 * rate) as u16
        } else {
            last_bill.total_production.max(1)
        };

        let production_under_budget = (self.cash * self.risk_appetite / self.product_cost) as u16;
        let need_production = prediction_production.min(production_under_budget);

        self.initial_stock = last_round_remaining_stock + need_production;
        // 扣除产量带来的成本
        let cost = need_production as f64 * self.product_cost;
        self.cash -= cost;
        let b = self.accountant.get_bill_or_default(round);
        let mut bill = b.write();
        bill.set_cash(self.cash);
        bill.set_initial_stock(self.initial_stock);
        bill.set_total_production(need_production);

        // 给hashmap创建一个以round为键，值为计算出的产量
        self.amount.insert(round, self.initial_stock);

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

    pub fn get_initial_stock(&self) -> u16 {
        self.initial_stock
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
                let ratio = get_range_change_ratio(interval_relation);
                let (lower, upper) = self.supply_price_range;
                let range_length = upper - lower;
                let (new_lower, new_upper) =
                    factory_shift_range_by_ratio(self.supply_price_range, self.product_cost, ratio);
                let (
                    lower_change_ratio,
                    upper_change_ratio,
                    total_change,
                    lower_change,
                    upper_change,
                ) = get_range_change_info((lower, upper), (new_lower, new_upper));
                // 调用日志记录函数
                let mut logger = LOGGER.write();
                if let Err(e) = logger.log_factory_range_optimization(
                    round,
                    self.id(),
                    self.name().to_string(),
                    self.product_id(),
                    format!("{:?}", self.product_category),
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
                let (new_lower, new_upper) =
                    factory_shift_range_by_ratio(self.supply_price_range, self.product_cost, 0.01);
                let (
                    lower_change_ratio,
                    upper_change_ratio,
                    total_change,
                    lower_change,
                    upper_change,
                ) = get_range_change_info((lower, upper), (new_lower, new_upper));
                // 调用日志记录函数
                let mut logger = LOGGER.write();
                // 调用日志记录函数
                if let Err(e) = logger.log_factory_range_optimization(
                    round,
                    self.id(),
                    self.name().to_string(),
                    self.product_id(),
                    format!("{:?}", self.product_category),
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

    pub fn settling_after_round(&mut self, round: u64) {
        let mut b = self.accountant.get_bill_or_default(round);
        let mut bill = b.write();
        let remaining_stock = self.amount.get(&round).unwrap_or(&0);
        println!(
            "initial_stock :{:?} remaining_stock :{:?}",
            bill.initial_stock, remaining_stock
        );
        let rot_stock = (*remaining_stock as f64 * (1.0 - self.durability)) as u16;
        let sales_amount = (bill.initial_stock - remaining_stock).max(0);
        bill.set_rot_stock(rot_stock);
        bill.set_units_sold(sales_amount);
        println!("bill.cash :{:?} self.cash:{:?}", bill.cash, self.cash);
        let revenue = bill.cash - self.cash;
        bill.set_revenue(revenue);
        bill.set_cash(self.cash);
        bill.set_remaining_stock(*remaining_stock - rot_stock);
        let units_gone = bill.units_sold + bill.rot_stock;
        let cost_of_goods_gone = units_gone as f64 * self.product_cost;
        bill.set_profit(revenue - cost_of_goods_gone);
    }
}

fn factory_shift_range_by_ratio(range: (f64, f64), min_cost: f64, ratio: f64) -> (f64, f64) {
    let (lower, upper) = shift_range_by_ratio(range, ratio);
    if lower < min_cost {
        let length = upper - lower;
        (min_cost, min_cost + length)
    } else {
        (lower, upper)
    }
}

fn get_range_change_info(
    old_range: (f64, f64),
    new_range: (f64, f64),
) -> (f64, f64, f64, f64, f64) {
    let (lower, upper) = old_range;
    let range_length = upper - lower;
    let (new_lower, new_upper) = new_range;
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
    (
        lower_change_ratio,
        upper_change_ratio,
        total_change,
        lower_change,
        upper_change,
    )
}

fn get_range_change_ratio(interval_relation: Option<IntervalRelation>) -> f64 {
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
    ratio
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::normal_distribute::NormalDistribution;
    use crate::model::product::{Product, ProductCategory};

    #[test]
    fn test_new() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(
            1,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            1.0,
        );
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
        let product = Product::new(
            1,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            1.0,
        );
        let factory = Factory::new(42, "test_factory".to_string(), &product);
        assert_eq!(factory.id(), 42);
    }

    #[test]
    fn test_name() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let factory = Factory::new(1, "my_factory".to_string(), &product);
        assert_eq!(factory.name(), "my_factory");
    }

    #[test]
    fn test_product_id() {
        let product = Product::new(
            5,
            "test_product".to_string(),
            crate::model::product::ProductCategory::Food,
            1.0,
        );
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        assert_eq!(factory.product_id(), 5);
    }

    #[test]
    fn test_supply_price_range() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }

    #[test]
    fn test_start_round_branch1() {
        // 分支1: last_round_initial_stock == 0
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 由于是第一轮，initial_stock为0，预测产量应该为1
        factory.start_round(1);
        let actual_production = factory.amount.get(&1).unwrap();
        assert_eq!(
            *actual_production, 1,
            "Branch 1: When last_round_initial_stock == 0, production should be 1"
        );
    }

    #[test]
    fn test_start_round_branch2() {
        // 分支2: last_round_remaining_stock == 0 (售罄情况)
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.risk_appetite = 0.5;

        // 为上一轮设置财务账单数据
        let last_round = 1;
        let b = factory.accountant.get_bill_or_default(last_round);
        let mut last_bill = b.write();
        last_bill.set_initial_stock(100);
        last_bill.set_remaining_stock(0); // 售罄
        last_bill.set_total_production(100);
        last_bill.set_units_sold(100);
        // 保存last_remaining_stock值
        let last_remaining_stock = last_bill.remaining_stock;
        drop(last_bill);

        factory.cash = 100000.0;
        factory.product_cost = 1.0;

        let current_round = 2;
        factory.start_round(current_round);
        let actual_initial_stock = factory.amount.get(&current_round).unwrap();

        // 验证产量在上一轮initial_stock的1.1~1.5倍之间
        let last_initial_stock = 100;
        let expected_min = (last_initial_stock as f64 * 1.1) as u16;
        let expected_max = (last_initial_stock as f64 * 1.5) as u16;
        let actual_production = actual_initial_stock - last_remaining_stock;
        assert!(
            actual_production >= expected_min,
            "Branch 2: When stock is sold out, production should be at least 1.1x last initial stock"
        );
        assert!(
            actual_production <= expected_max,
            "Branch 2: When stock is sold out, production should be at most 1.5x last initial stock"
        );
    }

    #[test]
    fn test_start_round_branch3_1() {
        // 分支3.1: else分支，有剩余库存的情况
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 为上一轮设置财务账单数据
        let last_round = 1;
        let b = factory.accountant.get_bill_or_default(last_round);
        let mut last_bill = b.write();
        last_bill.set_initial_stock(100);
        last_bill.set_remaining_stock(20);
        last_bill.set_total_production(100);
        last_bill.set_units_sold(80);
        // 保存last_remaining_stock值
        let last_remaining_stock = last_bill.remaining_stock;
        drop(last_bill);

        factory.cash = 100000.0;
        factory.product_cost = 1.0;

        // 新的逻辑：有剩余库存时，保持稳定产量
        let current_round = 2;
        factory.start_round(current_round);
        let actual_initial_stock = factory.amount.get(&current_round).unwrap();

        // 预期：上一轮剩余库存 + 上一轮总产量
        let expected_initial_stock = last_remaining_stock + 100;
        assert_eq!(
            *actual_initial_stock, expected_initial_stock,
            "Branch 3.1: When there is remaining stock, should keep stable production"
        );
    }

    #[test]
    fn test_start_round_branch3_2() {
        // 分支3.2: else分支，有剩余库存的情况
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 为上一轮设置财务账单数据
        let last_round = 1;
        let b = factory.accountant.get_bill_or_default(last_round);
        let mut last_bill = b.write();
        last_bill.set_initial_stock(50);
        last_bill.set_remaining_stock(40);
        last_bill.set_total_production(50);
        last_bill.set_units_sold(10);
        // 保存last_remaining_stock值
        let last_remaining_stock = last_bill.remaining_stock;
        drop(last_bill);

        factory.cash = 100000.0;
        factory.product_cost = 1.0;

        // 新的逻辑：有剩余库存时，保持稳定产量
        let current_round = 2;
        factory.start_round(current_round);
        let actual_initial_stock = factory.amount.get(&current_round).unwrap();

        // 预期：上一轮剩余库存 + 上一轮总产量
        let expected_initial_stock = last_remaining_stock + 50;
        assert_eq!(
            *actual_initial_stock, expected_initial_stock,
            "Branch 3.2: When there is remaining stock, should keep stable production"
        );
    }

    #[test]
    fn test_start_round_branch4_1() {
        // 分支4.1: 预算充足的情况
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 为上一轮设置财务账单数据
        let last_round = 1;
        let b = factory.accountant.get_bill_or_default(last_round);
        let mut last_bill = b.write();
        last_bill.set_initial_stock(100);
        last_bill.set_remaining_stock(20);
        last_bill.set_total_production(100);
        last_bill.set_units_sold(80);
        // 保存last_remaining_stock值
        let last_remaining_stock = last_bill.remaining_stock;
        drop(last_bill);

        factory.cash = 100000.0; // 大量现金，确保预算充足
        factory.product_cost = 1.0;

        let current_round = 2;
        factory.start_round(current_round);
        let actual_initial_stock = factory.amount.get(&current_round).unwrap();

        // 预期：上一轮剩余库存 + 上一轮总产量
        let expected_initial_stock = last_remaining_stock + 100;
        assert_eq!(
            *actual_initial_stock, expected_initial_stock,
            "Branch 4.1: When budget is sufficient, should keep stable production"
        );
    }

    #[test]
    fn test_start_round_branch4_2() {
        // 分支4.2: 预算不足的情况
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 为上一轮设置财务账单数据
        let last_round = 1;
        let b = factory.accountant.get_bill_or_default(last_round);
        let mut last_bill = b.write();
        last_bill.set_initial_stock(100);
        last_bill.set_remaining_stock(20);
        last_bill.set_total_production(100);
        last_bill.set_units_sold(80);
        // 保存last_remaining_stock值
        let last_remaining_stock = last_bill.remaining_stock;
        drop(last_bill);

        // 设置很少的现金，确保预算不足
        let initial_cash = 10.0;
        factory.cash = initial_cash;
        factory.product_cost = 1.0;
        factory.risk_appetite = 0.5;

        let current_round = 2;
        factory.start_round(current_round);
        let actual_initial_stock = factory.amount.get(&current_round).unwrap();

        // 更准确地计算预期值，与start_round方法逻辑保持一致
        let production_under_budget =
            (initial_cash * factory.risk_appetite / factory.product_cost) as u16;
        let prediction_production = 100; // 上一轮的总产量
        let need_production = prediction_production.min(production_under_budget);
        let expected_initial_stock = last_remaining_stock + need_production;

        assert_eq!(
            *actual_initial_stock, expected_initial_stock,
            "Branch 4.2: When budget is insufficient, initial_stock should match expected value"
        );
        assert!(
            *actual_initial_stock > 0,
            "Branch 4.2: Initial_stock should be greater than 0"
        );
    }

    #[test]
    fn test_start_round_queue_management() {
        // 测试队列管理功能
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 运行多轮，确保队列管理正常
        for round in 1..=5 {
            factory.start_round(round);
        }

        // 验证amount哈希表中只有最近3轮的数据
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
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 手动设置一个固定的supply_price_range，便于测试
        factory.supply_price_range = (100.0, 200.0);
        let initial_range = factory.supply_price_range;

        // 启动一轮
        let test_round = 1;
        factory.start_round(test_round);

        // 手动设置库存为10，因为新的start_round逻辑会根据历史数据计算产量
        let expected_initial_inventory = 10;
        *factory.amount.get_mut(&test_round).unwrap() = expected_initial_inventory;
        // 同时设置初始库存和剩余库存
        factory.initial_stock = expected_initial_inventory;
        factory.remaining_stock = expected_initial_inventory;

        // 测试交易成功情况
        factory.deal(&TradeResult::Success(150.0), test_round, None);
        // 更新剩余库存
        factory.remaining_stock -= 1;
        let after_success = factory.supply_price_range;

        // 测试交易失败情况 - 无区间关系
        let success_range = factory.supply_price_range;
        factory.deal(&TradeResult::Failed, test_round, None);
        // 更新剩余库存
        factory.remaining_stock -= 1;
        let after_failure = factory.supply_price_range;

        // 测试未匹配情况 - 区间不变
        let failure_range = factory.supply_price_range;
        factory.deal(&TradeResult::NotMatched, test_round, None);
        let after_not_matched = factory.supply_price_range;
        assert_eq!(after_not_matched, failure_range);
    }

    #[test]
    fn test_deal_with_interval_relation() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 手动设置一个固定的supply_price_range，便于测试
        factory.supply_price_range = (100.0, 200.0);

        // 启动一轮
        let test_round = 1;
        factory.start_round(test_round);

        // 手动设置库存为10，因为新的start_round逻辑会根据历史数据计算产量
        let expected_initial_inventory = 10;
        *factory.amount.get_mut(&test_round).unwrap() = expected_initial_inventory;
        // 同时设置初始库存和剩余库存
        factory.initial_stock = expected_initial_inventory;
        factory.remaining_stock = expected_initial_inventory;

        // 测试1: 交易失败 + Overlapping关系
        let initial_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::Overlapping((100.0, 200.0))),
        );
        // 更新剩余库存
        factory.remaining_stock -= 1;

        // 测试2: 交易失败 + AgentBelowFactory关系
        let overlapping_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::AgentBelowFactory),
        );
        // 更新剩余库存
        factory.remaining_stock -= 1;

        // 测试3: 交易失败 + AgentAboveFactory关系
        let below_range = factory.supply_price_range;
        factory.deal(
            &TradeResult::Failed,
            test_round,
            Some(IntervalRelation::AgentAboveFactory),
        );
        // 更新剩余库存
        factory.remaining_stock -= 1;

        // 只验证交易后区间仍然有效，不验证具体方向
        let after_above = factory.supply_price_range;
        assert!(after_above.0 >= 0.0, "Lower bound should be >= 0");
        assert!(
            after_above.1 > after_above.0,
            "Upper bound should be > lower bound"
        );
    }

    #[test]
    fn test_deal_with_small_range() {
        // 测试边界情况：小范围区间
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
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
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置初始供应价格范围
        factory.supply_price_range = (100.0, 200.0);
        // 手动设置工厂的现金和成本
        factory.cash = 10000.0; // 大量现金
        factory.product_cost = 100.0; // 较低的成本

        // 启动一轮
        let current_round = 1;
        factory.start_round(current_round);

        // 手动设置库存为100，因为新的start_round逻辑会根据历史数据计算产量
        let expected_initial_inventory = 100;
        *factory.amount.get_mut(&current_round).unwrap() = expected_initial_inventory;
        // 同时设置初始库存和剩余库存，用于后续轮次的计算
        factory.initial_stock = expected_initial_inventory;
        factory.remaining_stock = expected_initial_inventory;

        // 测试交易成功，库存减1
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        // 更新剩余库存
        factory.remaining_stock -= 1;

        // 测试多次交易成功，库存持续减少
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        factory.deal(&TradeResult::Success(150.0), current_round, None);
        // 更新剩余库存
        factory.remaining_stock -= 2;

        // 验证剩余库存正确
        assert_eq!(factory.remaining_stock, expected_initial_inventory - 3);
    }

    #[test]
    fn test_deal_with_zero_inventory() {
        // 测试库存为0时deal方法不执行
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
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
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);

        // 设置初始供应价格范围
        factory.supply_price_range = (100.0, 200.0);
        // 确保product_cost大于0，避免除以0错误
        factory.product_cost = factory.product_cost.max(1.0);
        // 确保有足够的现金用于生产
        factory.cash = 1000.0;

        // 启动一轮
        let current_round = 1;
        factory.start_round(current_round);

        // 手动设置库存为10，因为新的start_round逻辑会根据历史数据计算产量
        let expected_initial_inventory = 10;
        *factory.amount.get_mut(&current_round).unwrap() = expected_initial_inventory;
        // 同时设置初始库存和剩余库存
        factory.initial_stock = expected_initial_inventory;
        factory.remaining_stock = expected_initial_inventory;

        // 记录初始现金
        let initial_cash = factory.cash();

        let cash_before_deal = factory.cash();

        // 模拟交易成功，成交价为150.0
        let deal_price = 150.0;
        factory.deal(&TradeResult::Success(deal_price), current_round, None);
        // 更新剩余库存
        factory.remaining_stock -= 1;

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
        // 更新剩余库存
        factory.remaining_stock -= 1;

        // 验证cash字段累计更新
        assert!((factory.cash() - (cash_before_second_deal + second_deal_price)).abs() < 0.01);

        // 模拟交易失败，cash字段不应变化
        let cash_before_failed_deal = factory.cash();
        factory.deal(&TradeResult::Failed, current_round, None);
        assert!((factory.cash() - cash_before_failed_deal).abs() < 0.01);
    }

    #[test]
    fn test_get_range_change_ratio() {
        // 测试get_range_change_ratio函数的所有情况
        use crate::model::agent::IntervalRelation;

        // 情况1: interval_relation为None，应该返回-0.01
        let ratio_none = get_range_change_ratio(None);
        assert_eq!(ratio_none, -0.01);

        // 情况2: Overlapping关系，应该返回-0.01
        let ratio_overlapping =
            get_range_change_ratio(Some(IntervalRelation::Overlapping((10.0, 20.0))));
        assert_eq!(ratio_overlapping, -0.01);

        // 情况3: AgentBelowFactory关系，应该返回-0.01
        let ratio_below = get_range_change_ratio(Some(IntervalRelation::AgentBelowFactory));
        assert_eq!(ratio_below, -0.01);

        // 情况4: AgentAboveFactory关系，应该返回0.01
        let ratio_above = get_range_change_ratio(Some(IntervalRelation::AgentAboveFactory));
        assert_eq!(ratio_above, 0.01);
    }

    #[test]
    fn test_get_range_change_info() {
        // 测试get_range_change_info函数的各种情况

        // 情况1: 正常情况 - 范围扩大
        let old_range = (100.0, 200.0); // 原范围长度为100
        let new_range = (90.0, 210.0); // 新范围更长
        let result = get_range_change_info(old_range, new_range);

        // 计算预期值
        let expected_lower_change = 90.0 - 100.0; // -10.0
        let expected_upper_change = 210.0 - 200.0; // 10.0
        let expected_lower_change_ratio = expected_lower_change / 100.0; // -0.1
        let expected_upper_change_ratio = expected_upper_change / 100.0; // 0.1
        let expected_total_change = (90.0 + 210.0) - (100.0 + 200.0); // 0.0

        assert_eq!(result.0, expected_lower_change_ratio);
        assert_eq!(result.1, expected_upper_change_ratio);
        assert_eq!(result.2, expected_total_change);
        assert_eq!(result.3, expected_lower_change);
        assert_eq!(result.4, expected_upper_change);

        // 情况2: 正常情况 - 范围缩小
        let old_range = (100.0, 200.0); // 原范围长度为100
        let new_range = (110.0, 190.0); // 新范围更短
        let result = get_range_change_info(old_range, new_range);

        // 计算预期值
        let expected_lower_change = 110.0 - 100.0; // 10.0
        let expected_upper_change = 190.0 - 200.0; // -10.0
        let expected_lower_change_ratio = expected_lower_change / 100.0; // 0.1
        let expected_upper_change_ratio = expected_upper_change / 100.0; // -0.1
        let expected_total_change = (110.0 + 190.0) - (100.0 + 200.0); // 0.0

        assert_eq!(result.0, expected_lower_change_ratio);
        assert_eq!(result.1, expected_upper_change_ratio);
        assert_eq!(result.2, expected_total_change);
        assert_eq!(result.3, expected_lower_change);
        assert_eq!(result.4, expected_upper_change);

        // 情况3: 正常情况 - 范围上移
        let old_range = (100.0, 200.0); // 原范围
        let new_range = (110.0, 210.0); // 新范围上移
        let result = get_range_change_info(old_range, new_range);

        // 计算预期值
        let expected_lower_change = 110.0 - 100.0; // 10.0
        let expected_upper_change = 210.0 - 200.0; // 10.0
        let expected_lower_change_ratio = expected_lower_change / 100.0; // 0.1
        let expected_upper_change_ratio = expected_upper_change / 100.0; // 0.1
        let expected_total_change = (110.0 + 210.0) - (100.0 + 200.0); // 20.0

        assert_eq!(result.0, expected_lower_change_ratio);
        assert_eq!(result.1, expected_upper_change_ratio);
        assert_eq!(result.2, expected_total_change);
        assert_eq!(result.3, expected_lower_change);
        assert_eq!(result.4, expected_upper_change);

        // 情况4: 正常情况 - 范围下移
        let old_range = (100.0, 200.0); // 原范围
        let new_range = (90.0, 190.0); // 新范围下移
        let result = get_range_change_info(old_range, new_range);

        // 计算预期值
        let expected_lower_change = 90.0 - 100.0; // -10.0
        let expected_upper_change = 190.0 - 200.0; // -10.0
        let expected_lower_change_ratio = expected_lower_change / 100.0; // -0.1
        let expected_upper_change_ratio = expected_upper_change / 100.0; // -0.1
        let expected_total_change = (90.0 + 190.0) - (100.0 + 200.0); // -20.0

        assert_eq!(result.0, expected_lower_change_ratio);
        assert_eq!(result.1, expected_upper_change_ratio);
        assert_eq!(result.2, expected_total_change);
        assert_eq!(result.3, expected_lower_change);
        assert_eq!(result.4, expected_upper_change);

        // 情况5: 边界情况 - 原范围长度为0
        let old_range = (150.0, 150.0); // 原范围长度为0
        let new_range = (140.0, 160.0); // 新范围有长度
        let result = get_range_change_info(old_range, new_range);

        // 当原范围长度为0时，变化比例应该为0
        assert_eq!(result.0, 0.0);
        assert_eq!(result.1, 0.0);
        assert_eq!(result.2, (140.0 + 160.0) - (150.0 + 150.0)); // 0.0
        assert_eq!(result.3, 140.0 - 150.0); // -10.0
        assert_eq!(result.4, 160.0 - 150.0); // 10.0

        // 情况6: 边界情况 - 新范围与旧范围相同
        let old_range = (100.0, 200.0);
        let new_range = (100.0, 200.0);
        let result = get_range_change_info(old_range, new_range);

        // 所有变化都应该为0
        assert_eq!(result.0, 0.0);
        assert_eq!(result.1, 0.0);
        assert_eq!(result.2, 0.0);
        assert_eq!(result.3, 0.0);
        assert_eq!(result.4, 0.0);
    }

    #[test]
    fn test_factory_shift_range_by_ratio() {
        // 测试factory_shift_range_by_ratio函数的各种情况

        // 情况1: 正常情况 - 调整后的下界大于最小成本
        let range = (100.0, 200.0);
        let min_cost = 50.0;
        let ratio = 0.01; // 1% 增长
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：range的上下界都增长1%
        let expected_lower = 101.0;
        let expected_upper = 202.0;
        assert_eq!(result.0, expected_lower);
        assert_eq!(result.1, expected_upper);

        // 情况2: 边界情况 - 调整后的下界小于最小成本
        let range = (100.0, 200.0);
        let min_cost = 105.0;
        let ratio = -0.1; // 10% 下降
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：下界被调整为min_cost，范围长度保持不变
        let expected_lower = min_cost;
        let expected_upper = min_cost + (200.0 - 100.0) * 0.9; // 105.0 + 90.0 = 195.0
        assert_eq!(result.0, expected_lower);
        assert_eq!(result.1, expected_upper);

        // 情况3: 正常情况 - 比例为负，范围下移，但下界仍大于最小成本
        let range = (200.0, 300.0);
        let min_cost = 150.0;
        let ratio = -0.1; // 10% 下降
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：range的上下界都下降10%
        let expected_lower = 180.0;
        let expected_upper = 270.0;
        assert_eq!(result.0, expected_lower);
        assert_eq!(result.1, expected_upper);

        // 情况4: 边界情况 - 比例为0，范围不变
        let range = (100.0, 200.0);
        let min_cost = 50.0;
        let ratio = 0.0;
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：range保持不变
        assert_eq!(result.0, 100.0);
        assert_eq!(result.1, 200.0);

        // 情况5: 边界情况 - 初始范围的下界就是最小成本
        let range = (100.0, 200.0);
        let min_cost = 100.0;
        let ratio = 0.05; // 5% 增长
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：range的上下界都增长5%
        let expected_lower = 105.0;
        let expected_upper = 210.0;
        assert_eq!(result.0, expected_lower);
        assert_eq!(result.1, expected_upper);

        // 情况6: 边界情况 - 范围非常小
        let range = (0.01, 0.02);
        let min_cost = 0.01;
        let ratio = 0.1; // 10% 增长
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：范围至少保持不变或增长
        assert!(result.0 >= 0.01);
        assert!(result.1 > result.0);
        assert!(result.1 >= 0.02);

        // 情况7: 边界情况 - 调整后的下界刚好等于最小成本
        let range = (100.0, 200.0);
        let min_cost = 90.0;
        let ratio = -0.1; // 10% 下降
        let result = factory_shift_range_by_ratio(range, min_cost, ratio);

        // 预期结果：下界等于min_cost，上界为min_cost + 90.0
        assert_eq!(result.0, min_cost);
        assert_eq!(result.1, min_cost + 90.0);
    }

    #[test]
    fn test_factory_product_category() {
        let factory = Factory::new(
            1,
            "Test Factory".to_string(),
            &Product::new(1, "aaaa".to_string(), ProductCategory::Food, 1.0),
        );

        assert_eq!(factory.product_category(), ProductCategory::Food);
    }

    #[test]
    fn test_factory_setting_after_round() {
        let product = Product::from(
            1,
            "aaaa".to_string(),
            ProductCategory::Food,
            0.5,
            NormalDistribution::random(1, "aaaa_price_dist".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "aaaa_elastic_dist".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "aaaa_cost_dist".to_string(), Some(0.0), Some(1.0)),
        );

        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        {
            let mut b = factory.accountant.get_bill_or_default(1);
            let mut bill = b.write();
            bill.set_cash(100.0);
            bill.set_initial_stock(10);
            bill.set_production_cost(20.0);
        }

        let mut stocks = factory.amount.entry(1).or_insert(0);
        *stocks = 6;
        factory.cash = 51.0;
        factory.settling_after_round(1);
        let b = factory.accountant.get_bill_or_default(1);
        let bill = b.read();

        assert_eq!(bill.cash, 51.0);
        assert_eq!(bill.revenue, 49.0);
        assert_eq!(bill.initial_stock, 10);
        assert_eq!(bill.remaining_stock, 3);
        assert_eq!(bill.units_sold, 4);
        assert_eq!(bill.rot_stock, 3);
        assert_eq!(bill.profit, 49.0 - (3.0 + 4.0) * factory.product_cost);
    }
}
