mod accountant;
mod financial_bill;

use crate::logging::{LOGGER, log_factory_range_optimization};
use crate::model::agent::{IntervalRelation, TradeResult};
use crate::model::factory::accountant::Accountant;
use crate::model::factory::financial_bill::FinancialBill;
use crate::model::product::{Product, ProductCategory};
use crate::model::util::{round_to_nearest_cent, shift_range_by_ratio};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const BANKRUPTCY_ZERO_SALE_THRESHOLD: u64 = 200;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum FactoryStatus {
    #[default]
    Active,
    BrokeUp,
}

pub struct Factory {
    id: u64,
    name: String,
    product_id: u64,
    accountant: Accountant,
    product_category: ProductCategory,
    supply_price_range: (f64, f64),
    pub stock: u16,
    durability: f64,
    product_cost: f64,
    cash: f64,
    risk_appetite: f64,
    status: FactoryStatus,
    pub consecutive_zero_sale_ticks: u64,
    last_production_moment: u64,
    last_production_tick: u64,
    cycle_units_sold: u16,
    cycle_revenue: f64,
    cycle_initial_stock: u16,
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
            product_cost,
            durability: product.durability(),
            cash,
            stock: 0,
            risk_appetite: rng.gen_range(0.1..0.9),
            status: FactoryStatus::default(),
            consecutive_zero_sale_ticks: 0,
            last_production_moment: 0,
            last_production_tick: 0,
            cycle_units_sold: 0,
            cycle_revenue: 0.0,
            cycle_initial_stock: 0,
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

    pub fn offer_price(&self) -> f64 {
        if self.status == FactoryStatus::BrokeUp || self.stock == 0 {
            return 0.0;
        }
        let mut rng = rand::thread_rng();
        let (lower, upper) = self.supply_price_range;
        let price = rng.gen_range(lower..upper);
        round_to_nearest_cent(price)
    }

    pub fn resolve_trade(
        &mut self,
        result: &TradeResult,
        tick: u64,
        interval_relation: Option<IntervalRelation>,
    ) {
        if self.stock == 0 || self.status == FactoryStatus::BrokeUp {
            return;
        }

        match result {
            TradeResult::NotMatched | TradeResult::NotYet => {
                return;
            }
            TradeResult::Failed => {
                self.consecutive_zero_sale_ticks += 1;
                let ratio = get_range_change_ratio(interval_relation);
                let (lower, upper) = self.supply_price_range;
                let (new_lower, new_upper) =
                    factory_shift_range_by_ratio(self.supply_price_range, self.product_cost, ratio);
                let (
                    lower_change_ratio,
                    upper_change_ratio,
                    total_change,
                    lower_change,
                    upper_change,
                ) = get_range_change_info((lower, upper), (new_lower, new_upper));
                let mut logger = LOGGER.write();
                if let Err(e) = logger.log_factory_range_optimization(
                    tick,
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
                self.consecutive_zero_sale_ticks = 0;
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
                let mut logger = LOGGER.write();
                if let Err(e) = logger.log_factory_range_optimization(
                    tick,
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
                self.stock -= 1;
                self.cash += price;
                self.cycle_units_sold += 1;
                self.cycle_revenue += price;
            }
        }
    }

    fn settle_cycle(&mut self) {
        if self.last_production_moment == 0 {
            return;
        }
        let b = self.accountant.get_bill_or_default(self.last_production_moment);
        let mut bill = b.write();
        let units_gone = self.cycle_initial_stock.saturating_sub(self.stock);
        let rot_stock = (self.stock as f64 * (1.0 - self.durability)) as u16;
        bill.set_units_sold(self.cycle_units_sold);
        bill.set_revenue(self.cycle_revenue);
        bill.set_rot_stock(rot_stock);
        bill.set_remaining_stock(self.stock.saturating_sub(rot_stock));
        bill.set_cash(self.cash);
        let total_units_gone = self.cycle_units_sold + rot_stock;
        let cogs = total_units_gone as f64 * self.product_cost;
        bill.set_production_cost(cogs);
        bill.set_profit(self.cycle_revenue - cogs);
        self.accountant.add_bill(self.last_production_moment);
    }

    pub fn try_produce(&mut self, tick: u64) {
        if self.status == FactoryStatus::BrokeUp {
            return;
        }

        // Settle the previous production cycle
        self.settle_cycle();

        self.last_production_moment += 1;
        self.last_production_tick = tick;

        // Predict demand from recent sales in accountant's rolling window
        let total_bill = self.accountant.total_round_bill();
        let prediction = if total_bill.units_sold == 0 {
            1
        } else {
            total_bill.units_sold.max(1)
        };
        let production_under_budget = (self.cash * self.risk_appetite / self.product_cost) as u16;
        let need_production = prediction.min(production_under_budget.max(1));

        let cost = need_production as f64 * self.product_cost;
        self.cash -= cost;
        self.stock += need_production;

        // Reset cycle tracking
        self.cycle_units_sold = 0;
        self.cycle_revenue = 0.0;
        self.cycle_initial_stock = self.stock;

        // Record production in the new bill
        let b = self.accountant.get_bill_or_default(self.last_production_moment);
        let mut bill = b.write();
        bill.set_cash(self.cash);
        bill.set_initial_stock(self.stock);
        bill.set_total_production(need_production);
        bill.set_units_sold(0);
        bill.set_remaining_stock(self.stock);
    }

    pub fn apply_decay(&mut self) {
        if self.stock == 0 {
            return;
        }
        let rot = (self.stock as f64 * (1.0 - self.durability)) as u16;
        if rot > 0 {
            self.stock = self.stock.saturating_sub(rot);
        }
    }

    pub fn check_bankruptcy(&mut self) {
        if self.status == FactoryStatus::BrokeUp {
            return;
        }
        if self.consecutive_zero_sale_ticks > BANKRUPTCY_ZERO_SALE_THRESHOLD
            && self.stock == 0
            && self.cash < self.product_cost
        {
            self.status = FactoryStatus::BrokeUp;
        }
    }

    pub fn stock_ratio(&self) -> f64 {
        let total_bill = self.accountant.total_round_bill();
        let recent_production = total_bill.total_production.max(1) as f64;
        self.stock as f64 / recent_production
    }

    pub fn should_produce_on_cycle(&self, tick: u64) -> bool {
        tick - self.last_production_tick >= 100
    }

    pub fn start_production_thread(factory_arc: Arc<parking_lot::RwLock<Factory>>) {
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut local_tick: u64 = 0;
            loop {
                let sleep_ms = rng.gen_range(500..2000);
                thread::sleep(Duration::from_millis(sleep_ms));
                local_tick += 1;
                let mut factory = factory_arc.write();
                factory.apply_decay();
                factory.try_produce(local_tick);
                factory.check_bankruptcy();
            }
        });
    }

    pub fn get_round_bill(&self, round: u64) -> FinancialBill {
        let b = self.accountant.get_round_bill(round);
        b.expect("No bill found for round").clone()
    }

    pub fn status(&self) -> FactoryStatus {
        self.status
    }

    pub fn cogs_of_25_rounds(&self) -> f64 {
        let all_bills = self.accountant.total_round_bill();
        if all_bills.revenue == 0.0 {
            return 0.0;
        }
        (all_bills.revenue - all_bills.production_cost) / all_bills.revenue
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
            IntervalRelation::CashBurnedOut => {
                ratio = 0.0;
            }
        }
    }
    ratio
}

#[cfg(test)]
impl Factory {
    pub fn set_stock(&mut self, _round: u64, stock: u16) {
        self.stock = stock;
        self.cycle_initial_stock = stock;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::normal_distribute::NormalDistribution;
    use crate::model::product::{Product, ProductCategory};

    #[test]
    fn test_new() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let factory = Factory::new(1, "test_factory".to_string(), &product);

        assert_eq!(factory.id(), 1);
        assert_eq!(factory.name(), "test_factory");
        assert_eq!(factory.product_id(), 1);
        assert_eq!(factory.stock, 0);
        assert!(factory.cash() > 0.0);

        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }

    #[test]
    fn test_id() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
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
        let product = Product::new(5, "test_product".to_string(), ProductCategory::Food, 1.0);
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
    fn test_try_produce_first_cycle() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.cash = 1000.0;
        factory.product_cost = 10.0;
        factory.risk_appetite = 0.5;

        let initial_cash = factory.cash;
        factory.try_produce(0);

        assert!(factory.stock > 0, "Should produce at least 1 unit");
        assert!(factory.cash < initial_cash, "Cash should decrease after production");
        assert_eq!(factory.cycle_units_sold, 0);
    }

    #[test]
    fn test_try_produce_budget_constrained() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.cash = 5.0;
        factory.product_cost = 10.0;
        factory.risk_appetite = 0.5;

        factory.try_produce(0);

        // Budget allows at most (5 * 0.5 / 10) = 0 units, so min 1 is produced
        assert!(factory.stock >= 1);
        assert!(factory.cash < 5.0);
    }

    #[test]
    fn test_resolve_trade_success() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.supply_price_range = (100.0, 200.0);
        factory.stock = 10;
        factory.cycle_initial_stock = 10;

        let cash_before = factory.cash;
        factory.resolve_trade(&TradeResult::Success(150.0), 1, None);

        assert_eq!(factory.stock, 9);
        assert!((factory.cash - (cash_before + 150.0)).abs() < 0.01);
        assert_eq!(factory.cycle_units_sold, 1);
        assert_eq!(factory.consecutive_zero_sale_ticks, 0);
    }

    #[test]
    fn test_resolve_trade_failed() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.supply_price_range = (100.0, 200.0);
        factory.stock = 10;

        let cash_before = factory.cash;
        factory.resolve_trade(&TradeResult::Failed, 1, None);

        assert_eq!(factory.stock, 10, "Stock should not change on failed trade");
        assert!((factory.cash - cash_before).abs() < 0.01);
        assert_eq!(factory.consecutive_zero_sale_ticks, 1);
    }

    #[test]
    fn test_resolve_trade_not_matched() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.supply_price_range = (100.0, 200.0);
        factory.stock = 10;

        let range_before = factory.supply_price_range;
        factory.resolve_trade(&TradeResult::NotMatched, 1, None);

        assert_eq!(factory.stock, 10);
        assert_eq!(factory.supply_price_range, range_before);
    }

    #[test]
    fn test_resolve_trade_zero_stock() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.stock = 0;
        let range_before = factory.supply_price_range;

        factory.resolve_trade(&TradeResult::Success(150.0), 1, None);

        assert_eq!(factory.stock, 0);
        assert_eq!(factory.supply_price_range, range_before);
    }

    #[test]
    fn test_resolve_trade_broke_up() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.stock = 10;
        factory.status = FactoryStatus::BrokeUp;

        factory.resolve_trade(&TradeResult::Success(150.0), 1, None);

        assert_eq!(factory.stock, 10, "BrokeUp factory should not trade");
    }

    #[test]
    fn test_apply_decay() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 0.5);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.stock = 10;
        factory.durability = 0.5;

        factory.apply_decay();

        assert!(factory.stock < 10, "Stock should decrease from decay");
        assert!(factory.stock >= 5, "Stock should be at least half");
    }

    #[test]
    fn test_apply_decay_zero_stock() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 0.5);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.stock = 0;

        factory.apply_decay();

        assert_eq!(factory.stock, 0);
    }

    #[test]
    fn test_check_bankruptcy() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.consecutive_zero_sale_ticks = 201;
        factory.stock = 0;
        factory.cash = 0.0;
        factory.product_cost = 10.0;

        factory.check_bankruptcy();

        assert_eq!(factory.status(), FactoryStatus::BrokeUp);
    }

    #[test]
    fn test_check_bankruptcy_not_yet() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.consecutive_zero_sale_ticks = 50;
        factory.stock = 0;
        factory.cash = 0.0;
        factory.product_cost = 10.0;

        factory.check_bankruptcy();

        assert_eq!(factory.status(), FactoryStatus::Active);
    }

    #[test]
    fn test_offer_price() {
        let product = Product::from(
            1, "test_product".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        factory.supply_price_range = (10.0, 20.0);
        factory.stock = 10;

        for _ in 0..50 {
            let price = factory.offer_price();
            assert!(price >= 10.0 && price <= 20.0);
        }
    }

    #[test]
    fn test_offer_price_broke_up() {
        let product = Product::from(
            1, "test_product".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        factory.stock = 10;
        factory.status = FactoryStatus::BrokeUp;

        assert_eq!(factory.offer_price(), 0.0);
    }

    #[test]
    fn test_offer_price_zero_stock() {
        let product = Product::from(
            1, "test_product".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        factory.supply_price_range = (10.0, 20.0);
        factory.stock = 0;

        assert_eq!(factory.offer_price(), 0.0);
    }

    #[test]
    fn test_stock_ratio() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.stock = 5;
        factory.cash = 1000.0;
        factory.product_cost = 10.0;
        factory.risk_appetite = 0.5;

        // Produce first to set up accountant
        factory.try_produce(0);

        let ratio = factory.stock_ratio();
        assert!(ratio > 0.0);
    }

    #[test]
    fn test_should_produce_on_cycle() {
        let product = Product::new(1, "test_product".to_string(), ProductCategory::Food, 1.0);
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        factory.last_production_tick = 0;

        assert!(!factory.should_produce_on_cycle(50));
        assert!(factory.should_produce_on_cycle(100));
        assert!(factory.should_produce_on_cycle(200));
    }

    #[test]
    fn test_factory_product_category() {
        let factory = Factory::new(
            1, "Test Factory".to_string(),
            &Product::new(1, "aaaa".to_string(), ProductCategory::Food, 1.0),
        );
        assert_eq!(factory.product_category(), ProductCategory::Food);
    }

    #[test]
    fn test_factory_get_round_bill() {
        let product = Product::from(
            1, "aaaa".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);
        {
            let b = factory.accountant.get_bill_or_default(1);
            let mut bill = b.write();
            bill.set_cash(100.0);
            bill.set_initial_stock(10);
            bill.set_production_cost(20.0);
        }
        let bill = factory.get_round_bill(1);
        assert_eq!(bill.cash, 100.0);
        assert_eq!(bill.initial_stock, 10);
        assert_eq!(bill.production_cost, 20.0);
    }

    #[test]
    fn test_cogs_of_25_rounds() {
        let product = Product::from(
            1, "test_product".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let mut factory = Factory::new(1, "Test Factory".to_string(), &product);

        let cogs_1 = factory.cogs_of_25_rounds();
        assert_eq!(cogs_1, 0.0);

        let bill1 = factory.accountant.get_bill_or_default(1);
        {
            let mut bill_write = bill1.write();
            bill_write.set_revenue(150.0);
            bill_write.set_production_cost(100.0);
        }
        factory.accountant.add_bill(1);

        let cogs_2 = factory.cogs_of_25_rounds();
        assert_eq!(cogs_2, 0.3333333333333333);

        let bill2 = factory.accountant.get_bill_or_default(2);
        {
            let mut bill_write = bill2.write();
            bill_write.set_revenue(200.0);
            bill_write.set_production_cost(50.0);
        }
        factory.accountant.add_bill(2);

        let cogs_3 = factory.cogs_of_25_rounds();
        assert_eq!(cogs_3, 0.5714285714285714);
    }

    #[test]
    fn test_status() {
        let product = Product::from(
            1, "test_product".to_string(), ProductCategory::Food, 0.5,
            NormalDistribution::random(1, "price".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "elastic".to_string(), Some(0.0), Some(1.0)),
            NormalDistribution::random(1, "cost".to_string(), Some(0.0), Some(1.0)),
        );
        let factory = Factory::new(1, "Test Factory".to_string(), &product);
        assert_eq!(factory.status(), FactoryStatus::Active);
    }

    #[test]
    fn test_get_range_change_ratio() {
        // 测试get_range_change_ratio函数的所有情况
        use crate::model::agent::IntervalRelation;

        // 情况1: interval_relation为None，应该返回-0.01
        let ratio_none = get_range_change_ratio(None);
        assert_eq!(ratio_none, -0.01);

        // 情况2: Overlapping关系，应该返回-0.01
        let ratio_overlapping = get_range_change_ratio(Some(IntervalRelation::Overlapping((10.0))));
        assert_eq!(ratio_overlapping, -0.01);

        // 情况3: AgentBelowFactory关系，应该返回-0.01
        let ratio_below = get_range_change_ratio(Some(IntervalRelation::AgentBelowFactory));
        assert_eq!(ratio_below, -0.01);

        // 情况4: AgentAboveFactory关系，应该返回0.01
        let ratio_above = get_range_change_ratio(Some(IntervalRelation::AgentAboveFactory));
        assert_eq!(ratio_above, 0.01);

        // 情况5: CashBurnedOut关系，应该返回0.0
        let ratio_burned_out = get_range_change_ratio(Some(IntervalRelation::CashBurnedOut));
        assert_eq!(ratio_burned_out, 0.0);
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
}
