use crate::model::agent::Agent;
use crate::model::agent::TradeResult;
use crate::model::factory::Factory;
use crate::model::product::Product;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// 交易日志结构体
pub struct TradeLog {
    pub timestamp: i64,
    pub round: u64,
    pub trade_id: u64,
    pub task_id: String,
    pub agent_id: u64,
    pub agent_name: String,
    pub agent_cash: f64,
    pub factory_id: u64,
    pub factory_name: String,
    pub product_id: u64,
    pub product_name: String,
    pub trade_result: String,
    pub interval_relation: String,
    pub price: f64,
    pub factory_supply_range_lower: f64,
    pub factory_supply_range_upper: f64,
    pub factory_stock: i16,
    pub agent_pref_original_price: f64,
    pub agent_pref_original_elastic: f64,
    pub agent_pref_current_price: f64,
    pub agent_pref_current_range_lower: f64,
    pub agent_pref_current_range_upper: f64,
}

impl TradeLog {
    pub fn new(
        timestamp:i64,
        round: u64,
        trade_id: u64,
        task_id: String,
        agent_id: u64,
        agent_name: String,
        agent_cash: f64,
        agent_pref_original_price:f64,
        agent_pref_original_elastic:f64,
        agent_pref_current_price:f64,
        agent_pref_current_range_lower:f64,
        agent_pref_current_range_upper:f64,
        factory: &Factory,
        product: &Product,
        trade_result: &TradeResult,
        interval_relation: &str,
    ) -> Self {
        let (result_str, price) = match trade_result {
            TradeResult::NotMatched => ("NotMatched", None),
            TradeResult::Failed => ("Failed", None),
            TradeResult::Success(p) => ("Success", Some(*p)),
            TradeResult::NotYet => ("NotYet", None),
        };

        let (lower, upper) = factory.supply_price_range();

        TradeLog {
            timestamp,
            round,
            trade_id,
            task_id,
            agent_id,
            agent_name,
            agent_cash,
            factory_id: factory.id(),
            factory_name: factory.name().to_string(),
            product_id: product.id(),
            product_name: product.name().to_string(),
            trade_result: result_str.to_string(),
            interval_relation: interval_relation.to_string(),
            price:price.unwrap_or(-1.0),
            factory_supply_range_lower: lower,
            factory_supply_range_upper: upper,
            factory_stock: factory.get_stock(round),
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
        }
    }
}

pub fn log_trade(
    timestamp:i64,
    round: u64,
    trade_id: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    agent_cash: f64,
    agent_pref_original_price:f64,
    agent_pref_original_elastic:f64,
    agent_pref_current_price:f64,
    agent_pref_current_range_lower:f64,
    agent_pref_current_range_upper:f64,
    factory: &Factory,
    product: &Product,
    trade_result: &TradeResult,
    interval_relation: &str,
) -> String {
    let log = TradeLog::new(
        timestamp,
        round,
        trade_id,
        task_id,
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
        trade_result,
        interval_relation,
    );

    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO trade_logs (
                    timestamp, round, trade_id, task_id, agent_id, agent_name, agent_cash,
                    factory_id, factory_name, product_id, product_name, trade_result, interval_relation, price,
                    factory_supply_range_lower, factory_supply_range_upper, factory_stock,
                    agent_pref_original_price, agent_pref_original_elastic, agent_pref_current_price,
                    agent_pref_current_range_lower, agent_pref_current_range_upper
                ) VALUES (
                    {}, {}, {}, '{}', {}, '{}', {},
                    {}, '{}', {}, '{}', '{}', '{}', {},
                    {}, {}, {},
                    {}, {}, {},
                    {}, {}
                )
            "#,
        log.timestamp,
        log.round,
        log.trade_id,
        log.task_id,
        log.agent_id,
        log.agent_name,
        log.agent_cash,
        log.factory_id,
        log.factory_name,
        log.product_id,
        log.product_name,
        log.trade_result,
        log.interval_relation,
        log.price,
        log.factory_supply_range_lower,
        log.factory_supply_range_upper,
        log.factory_stock,
        log.agent_pref_original_price,
        log.agent_pref_original_elastic,
        log.agent_pref_current_price,
        log.agent_pref_current_range_lower,
        log.agent_pref_current_range_upper,
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::agent::Agent;
    use crate::model::factory::Factory;
    use crate::model::product::Product;
    use parking_lot::RwLock;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_trade_log_new_with_success_result() {
        // 测试TradeLog::new方法，交易成功的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 100;
        let task_id = "test_task_567".to_string();
        let interval_relation = "overlap";
        let trade_result = TradeResult::Success(95.5);

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let log = TradeLog::new(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        // 验证时间戳是否为我们传入的时间
        assert_eq!(log.timestamp, timestamp);

        // 验证其他字段
        assert_eq!(log.round, round);
        assert_eq!(log.trade_id, trade_id);
        assert_eq!(log.task_id, task_id);
        assert_eq!(log.agent_id, 1);
        assert_eq!(log.agent_name, "TestAgent".to_string());
        assert_eq!(log.factory_id, 1);
        assert_eq!(log.factory_name, "TestFactory".to_string());
        assert_eq!(log.product_id, 1);
        assert_eq!(log.product_name, "TestProduct".to_string());
        assert_eq!(log.trade_result, "Success".to_string());
        assert_eq!(log.interval_relation, interval_relation.to_string());
        assert_eq!(log.price, 95.5);
    }

    #[test]
    fn test_trade_log_new_with_failed_result() {
        // 测试TradeLog::new方法，交易失败的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 101;
        let task_id = "test_task_567".to_string();
        let interval_relation = "disjoint";
        let trade_result = TradeResult::Failed;

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let log = TradeLog::new(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        assert_eq!(log.trade_result, "Failed".to_string());
        assert_eq!(log.price, -1.0);
        assert_eq!(log.interval_relation, interval_relation.to_string());
    }

    #[test]
    fn test_trade_log_new_with_not_matched_result() {
        // 测试TradeLog::new方法，交易不匹配的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 102;
        let task_id = "test_task_567".to_string();
        let interval_relation = "adjacent";
        let trade_result = TradeResult::NotMatched;

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let log = TradeLog::new(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        assert_eq!(log.trade_result, "NotMatched".to_string());
        assert_eq!(log.price, -1.0);
    }

    #[test]
    fn test_log_trade_with_success_result() {
        // 测试log_trade函数，交易成功的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 100;
        let task_id = "test_task_567".to_string();
        let interval_relation = "overlap";
        let trade_result = TradeResult::Success(95.5);

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let sql = log_trade(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        // 验证SQL包含正确的表名和字段
        assert!(sql.contains("INSERT INTO trade_logs"));
        assert!(sql.contains("timestamp"));
        assert!(sql.contains("round"));
        assert!(sql.contains("trade_id"));
        assert!(sql.contains("task_id"));
        assert!(sql.contains("agent_id"));
        assert!(sql.contains("agent_name"));
        assert!(sql.contains("agent_cash"));
        assert!(sql.contains("factory_id"));
        assert!(sql.contains("factory_name"));
        assert!(sql.contains("product_id"));
        assert!(sql.contains("product_name"));
        assert!(sql.contains("trade_result"));
        assert!(sql.contains("interval_relation"));
        assert!(sql.contains("price"));

        // 验证SQL包含正确的值
        assert!(sql.contains(&round.to_string()));
        assert!(sql.contains(&trade_id.to_string()));
        assert!(sql.contains(&task_id));
        assert!(sql.contains(&"TestAgent"));
        assert!(sql.contains(&"TestFactory"));
        assert!(sql.contains(&"TestProduct"));
        assert!(sql.contains(&"Success"));
        assert!(sql.contains(&"overlap"));
        assert!(sql.contains(&"95.5"));
    }

    #[test]
    fn test_log_trade_with_failed_result() {
        // 测试log_trade函数，交易失败的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 101;
        let task_id = "test_task_567".to_string();
        let interval_relation = "disjoint";
        let trade_result = TradeResult::Failed;

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let sql = log_trade(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        // 验证SQL包含正确的表名和字段
        assert!(sql.contains("INSERT INTO trade_logs"));
        assert!(sql.contains("trade_result"));
        assert!(sql.contains("interval_relation"));
        assert!(sql.contains("price"));

        // 验证SQL包含正确的值
        assert!(sql.contains(&"Failed"));
        assert!(sql.contains(&"disjoint"));
        assert!(sql.contains(&"-1")); // 验证失败情况下价格使用默认值-1
    }

    #[test]
    fn test_log_trade_with_not_matched_result() {
        // 测试log_trade函数，交易不匹配的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 102;
        let task_id = "test_task_567".to_string();
        let interval_relation = "adjacent";
        let trade_result = TradeResult::NotMatched;

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let sql = log_trade(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        // 验证SQL包含正确的值
        assert!(sql.contains(&"NotMatched"));
        assert!(sql.contains(&"adjacent"));
        assert!(sql.contains(&"-1")); // 验证不匹配情况下价格使用默认值-1
    }

    #[test]
    fn test_log_trade_with_not_yet_result() {
        // 测试log_trade函数，交易尚未进行的情况
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        let round = 5;
        let trade_id = 103;
        let task_id = "test_task_567".to_string();
        let interval_relation = "unknown";
        let trade_result = TradeResult::NotYet;

        // 创建测试用的Product
        let product = Product::new(1, "TestProduct".to_string());
        // 创建测试用的Factory，使用正确的参数
        let factory = Factory::new(1, "TestFactory".to_string(), &product);

        let sql = log_trade(
            timestamp,
            round,
            trade_id,
            task_id.clone(),
            1, // agent_id
            "TestAgent".to_string(), // agent_name
            1000.0, // agent_cash
            100.0, // agent_pref_original_price
            0.5, // agent_pref_original_elastic
            98.0, // agent_pref_current_price
            90.0, // agent_pref_current_range_lower
            110.0, // agent_pref_current_range_upper
            &factory,
            &product,
            &trade_result,
            interval_relation,
        );

        // 验证SQL包含正确的值
        assert!(sql.contains(&"NotYet"));
        assert!(sql.contains(&"unknown"));
        assert!(sql.contains(&"-1")); // 验证尚未进行情况下价格使用默认值-1
    }
}
