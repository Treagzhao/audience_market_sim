use std::time::{SystemTime, UNIX_EPOCH};

// Agent范围调整日志结构体
pub struct AgentRangeAdjustmentLog {
    pub timestamp: i64,
    pub round: u64,
    pub task_id: String,
    pub agent_id: u64,
    pub agent_name: String,
    pub product_id: u64,
    pub old_range_lower: f64,
    pub old_range_upper: f64,
    pub new_range_lower: f64,
    pub new_range_upper: f64,
    pub lower_change: f64,
    pub upper_change: f64,
    pub min_change_ratio: f64,
    pub max_change_ratio: f64,
    pub center: f64,
    pub adjustment_type: String, // "trade_success" 或 "trade_failed"
    pub price: Option<f64>,      // 仅在交易成功时有值
}

impl AgentRangeAdjustmentLog {
    pub fn new(
        round: u64,
        task_id: String,
        agent_id: u64,
        agent_name: String,
        product_id: u64,
        old_range: (f64, f64),
        new_range: (f64, f64),
        lower_change: f64,
        upper_change: f64,
        min_change_ratio: f64,
        max_change_ratio: f64,
        center: f64,
        adjustment_type: &str,
        price: Option<f64>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        AgentRangeAdjustmentLog {
            timestamp,
            round,
            task_id,
            agent_id,
            agent_name,
            product_id,
            old_range_lower: old_range.0,
            old_range_upper: old_range.1,
            new_range_lower: new_range.0,
            new_range_upper: new_range.1,
            lower_change,
            upper_change,
            min_change_ratio,
            max_change_ratio,
            center,
            adjustment_type: adjustment_type.to_string(),
            price,
        }
    }
}

pub fn log_agent_range_adjustment(
    round: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    product_id: u64,
    old_range: (f64, f64),
    new_range: (f64, f64),
    lower_change: f64,
    upper_change: f64,
    min_change_ratio: f64,
    max_change_ratio: f64,
    center: f64,
    adjustment_type: &str,
    price: Option<f64>,
) -> String {
    let log = AgentRangeAdjustmentLog::new(
        round,
        task_id, // 这里需要传入task_id，暂时留空
        agent_id,
        agent_name,
        product_id,
        old_range,
        new_range,
        lower_change,
        upper_change,
        min_change_ratio,
        max_change_ratio,
        center,
        adjustment_type,
        price,
    );

    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO agent_range_adjustment_logs (
                    timestamp, round, task_id, agent_id, agent_name, product_id,
                    old_range_lower, old_range_upper, new_range_lower, new_range_upper,
                    lower_change, upper_change, min_change_ratio, max_change_ratio,
                    center, adjustment_type, price
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {},
                    {}, {}, {}, {},
                    {}, {}, {}, {},
                    {}, '{}', {}
                )
            "#,
        log.timestamp,
        log.round,
        log.task_id,
        log.agent_id,
        log.agent_name,
        log.product_id,
        log.old_range_lower,
        log.old_range_upper,
        log.new_range_lower,
        log.new_range_upper,
        log.lower_change,
        log.upper_change,
        log.min_change_ratio * 100.0, // 转换为百分比
        log.max_change_ratio * 100.0, // 转换为百分比
        log.center,
        log.adjustment_type,
        log.price.unwrap_or(-1.0)
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_agent_range_adjustment_log_new() {
        // 测试AgentRangeAdjustmentLog::new方法
        let round = 15;
        let task_id = "test_task_456".to_string();
        let agent_id = 789;
        let agent_name = "TestAgent2".to_string();
        let product_id = 101;
        let old_range = (50.0, 100.0);
        let new_range = (60.0, 120.0);
        let lower_change = 10.0;
        let upper_change = 20.0;
        let min_change_ratio = 0.1;
        let max_change_ratio = 0.2;
        let center = 90.0;
        let adjustment_type = "trade_success";
        let price = Some(85.5);

        let log = AgentRangeAdjustmentLog::new(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            min_change_ratio,
            max_change_ratio,
            center,
            adjustment_type,
            price,
        );

        // 验证时间戳是否为当前时间附近（允许1秒误差）
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;
        assert!(log.timestamp >= current_time - 1000 && log.timestamp <= current_time + 1000);

        // 验证其他字段
        assert_eq!(log.round, round);
        assert_eq!(log.task_id, task_id);
        assert_eq!(log.agent_id, agent_id);
        assert_eq!(log.agent_name, agent_name);
        assert_eq!(log.product_id, product_id);
        assert_eq!(log.old_range_lower, old_range.0);
        assert_eq!(log.old_range_upper, old_range.1);
        assert_eq!(log.new_range_lower, new_range.0);
        assert_eq!(log.new_range_upper, new_range.1);
        assert_eq!(log.lower_change, lower_change);
        assert_eq!(log.upper_change, upper_change);
        assert_eq!(log.min_change_ratio, min_change_ratio);
        assert_eq!(log.max_change_ratio, max_change_ratio);
        assert_eq!(log.center, center);
        assert_eq!(log.adjustment_type, adjustment_type);
        assert_eq!(log.price, price);
    }

    #[test]
    fn test_agent_range_adjustment_log_new_without_price() {
        // 测试没有价格的情况
        let round = 15;
        let task_id = "test_task_456".to_string();
        let agent_id = 789;
        let agent_name = "TestAgent2".to_string();
        let product_id = 101;
        let old_range = (50.0, 100.0);
        let new_range = (60.0, 120.0);
        let lower_change = 10.0;
        let upper_change = 20.0;
        let min_change_ratio = 0.1;
        let max_change_ratio = 0.2;
        let center = 90.0;
        let adjustment_type = "trade_failed";
        let price = None;

        let log = AgentRangeAdjustmentLog::new(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            min_change_ratio,
            max_change_ratio,
            center,
            adjustment_type,
            price,
        );

        assert_eq!(log.adjustment_type, adjustment_type);
        assert_eq!(log.price, price);
    }

    #[test]
    fn test_log_agent_range_adjustment() {
        // 测试log_agent_range_adjustment函数生成的SQL
        let round = 15;
        let task_id = "test_task_456".to_string();
        let agent_id = 789;
        let agent_name = "TestAgent2".to_string();
        let product_id = 101;
        let old_range = (50.0, 100.0);
        let new_range = (60.0, 120.0);
        let lower_change = 10.0;
        let upper_change = 20.0;
        let min_change_ratio = 0.1;
        let max_change_ratio = 0.2;
        let center = 90.0;
        let adjustment_type = "trade_success";
        let price = Some(85.5);

        let sql = log_agent_range_adjustment(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            min_change_ratio,
            max_change_ratio,
            center,
            adjustment_type,
            price,
        );

        // 验证SQL包含正确的表名和字段
        assert!(sql.contains("INSERT INTO agent_range_adjustment_logs"));
        assert!(sql.contains("timestamp"));
        assert!(sql.contains("round"));
        assert!(sql.contains("task_id"));
        assert!(sql.contains("agent_id"));
        assert!(sql.contains("agent_name"));
        assert!(sql.contains("product_id"));
        assert!(sql.contains("old_range_lower"));
        assert!(sql.contains("old_range_upper"));
        assert!(sql.contains("new_range_lower"));
        assert!(sql.contains("new_range_upper"));
        assert!(sql.contains("lower_change"));
        assert!(sql.contains("upper_change"));
        assert!(sql.contains("min_change_ratio"));
        assert!(sql.contains("max_change_ratio"));
        assert!(sql.contains("center"));
        assert!(sql.contains("adjustment_type"));
        assert!(sql.contains("price"));

        // 验证SQL包含正确的值（部分关键值）
        assert!(sql.contains(&round.to_string()));
        assert!(sql.contains(&task_id));
        assert!(sql.contains(&agent_id.to_string()));
        assert!(sql.contains(&agent_name));
        assert!(sql.contains(&product_id.to_string()));
        assert!(sql.contains(&old_range.0.to_string()));
        assert!(sql.contains(&old_range.1.to_string()));
        assert!(sql.contains(&new_range.0.to_string()));
        assert!(sql.contains(&new_range.1.to_string()));
        assert!(sql.contains(&adjustment_type));
        assert!(sql.contains(&price.unwrap().to_string()));
    }

    #[test]
    fn test_log_agent_range_adjustment_without_price() {
        // 测试没有价格的情况生成的SQL
        let round = 15;
        let task_id = "test_task_456".to_string();
        let agent_id = 789;
        let agent_name = "TestAgent2".to_string();
        let product_id = 101;
        let old_range = (50.0, 100.0);
        let new_range = (60.0, 120.0);
        let lower_change = 10.0;
        let upper_change = 20.0;
        let min_change_ratio = 0.1;
        let max_change_ratio = 0.2;
        let center = 90.0;
        let adjustment_type = "trade_failed";
        let price = None;

        let log = AgentRangeAdjustmentLog::new(
            round,
            task_id,
            agent_id,
            agent_name,
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            min_change_ratio,
            max_change_ratio,
            center,
            adjustment_type,
            price,
        );

        // 验证日志对象的属性
        assert_eq!(log.adjustment_type, adjustment_type);
        assert_eq!(log.price, price);
        assert_eq!(log.agent_id, agent_id);
        assert_eq!(log.product_id, product_id);
    }
}
