use std::time::{SystemTime, UNIX_EPOCH};

// Agent需求删除日志结构体
pub struct AgentDemandRemovalLog {
    pub timestamp: i64,
    pub round: u64,
    pub task_id: String,
    pub agent_id: u64,
    pub agent_name: String,
    pub product_id: u64,
    pub agent_cash: f64,
    pub agent_pref_original_price: Option<f64>,
    pub agent_pref_original_elastic: Option<f64>,
    pub agent_pref_current_price: Option<f64>,
    pub agent_pref_current_range_lower: Option<f64>,
    pub agent_pref_current_range_upper: Option<f64>,
    pub removal_reason: String,
}

impl AgentDemandRemovalLog {
    pub fn new(
        round: u64,
        task_id: String,
        agent_id: u64,
        agent_name: String,
        product_id: u64,
        agent_cash: f64,
        agent_pref_original_price: Option<f64>,
        agent_pref_original_elastic: Option<f64>,
        agent_pref_current_price: Option<f64>,
        agent_pref_current_range_lower: Option<f64>,
        agent_pref_current_range_upper: Option<f64>,
        removal_reason: &str,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        AgentDemandRemovalLog {
            timestamp,
            round,
            task_id,
            agent_id,
            agent_name,
            product_id,
            agent_cash,
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
            removal_reason: removal_reason.to_string(),
        }
    }
}

// 生成创建表的SQL语句
pub fn generate_create_table_sql() -> String {
    r#"
    CREATE TABLE IF NOT EXISTS agent_demand_removal_logs (
        id INT AUTO_INCREMENT PRIMARY KEY,
        timestamp BIGINT NOT NULL,
        round INT UNSIGNED NOT NULL,
        task_id VARCHAR(255) NOT NULL,
        agent_id INT UNSIGNED NOT NULL,
        agent_name VARCHAR(255) NOT NULL,
        product_id INT UNSIGNED NOT NULL,
        agent_cash DOUBLE NOT NULL,
        agent_pref_original_price DOUBLE,
        agent_pref_original_elastic DOUBLE,
        agent_pref_current_price DOUBLE,
        agent_pref_current_range_lower DOUBLE,
        agent_pref_current_range_upper DOUBLE,
        removal_reason VARCHAR(255) NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#
    .to_string()
}

pub fn log_agent_demand_removal(
    round: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    product_id: u64,
    agent_cash: f64,
    agent_pref_original_price: Option<f64>,
    agent_pref_original_elastic: Option<f64>,
    agent_pref_current_price: Option<f64>,
    agent_pref_current_range_lower: Option<f64>,
    agent_pref_current_range_upper: Option<f64>,
    removal_reason: &str,
) -> String {
    let log = AgentDemandRemovalLog::new(
        round,
        task_id.clone(),
        agent_id,
        agent_name.clone(),
        product_id,
        agent_cash,
        agent_pref_original_price,
        agent_pref_original_elastic,
        agent_pref_current_price,
        agent_pref_current_range_lower,
        agent_pref_current_range_upper,
        removal_reason,
    );

    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO agent_demand_removal_logs (
                    timestamp, round, task_id, agent_id, agent_name, product_id, agent_cash,
                    agent_pref_original_price, agent_pref_original_elastic, agent_pref_current_price,
                    agent_pref_current_range_lower, agent_pref_current_range_upper, removal_reason
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {}, {},
                    {}, {}, {},
                    {}, {}, '{}'
                )
            "#,
        log.timestamp,
        log.round,
        log.task_id,
        log.agent_id,
        log.agent_name,
        log.product_id,
        log.agent_cash,
        log.agent_pref_original_price.unwrap_or(-1.0),
        log.agent_pref_original_elastic.unwrap_or(-1.0),
        log.agent_pref_current_price.unwrap_or(-1.0),
        log.agent_pref_current_range_lower.unwrap_or(-1.0),
        log.agent_pref_current_range_upper.unwrap_or(-1.0),
        log.removal_reason
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_agent_demand_removal_log_new() {
        // 测试AgentDemandRemovalLog::new方法
        let round = 20;
        let task_id = "test_task_789".to_string();
        let agent_id = 123;
        let agent_name = "TestAgent3".to_string();
        let product_id = 202;
        let agent_cash = 500.75;
        let agent_pref_original_price = Some(100.0);
        let agent_pref_original_elastic = Some(1.5);
        let agent_pref_current_price = Some(110.0);
        let agent_pref_current_range_lower = Some(90.0);
        let agent_pref_current_range_upper = Some(120.0);
        let removal_reason = "out_of_cash";

        let log = AgentDemandRemovalLog::new(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            product_id,
            agent_cash,
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
            removal_reason,
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
        assert_eq!(log.agent_cash, agent_cash);
        assert_eq!(log.agent_pref_original_price, agent_pref_original_price);
        assert_eq!(log.agent_pref_original_elastic, agent_pref_original_elastic);
        assert_eq!(log.agent_pref_current_price, agent_pref_current_price);
        assert_eq!(
            log.agent_pref_current_range_lower,
            agent_pref_current_range_lower
        );
        assert_eq!(
            log.agent_pref_current_range_upper,
            agent_pref_current_range_upper
        );
        assert_eq!(log.removal_reason, removal_reason);
    }

    #[test]
    fn test_agent_demand_removal_log_new_without_preferences() {
        // 测试没有偏好信息的情况
        let round = 20;
        let task_id = "test_task_789".to_string();
        let agent_id = 123;
        let agent_name = "TestAgent3".to_string();
        let product_id = 202;
        let agent_cash = 500.75;
        let agent_pref_original_price = None;
        let agent_pref_original_elastic = None;
        let agent_pref_current_price = None;
        let agent_pref_current_range_lower = None;
        let agent_pref_current_range_upper = None;
        let removal_reason = "no_preference";

        let log = AgentDemandRemovalLog::new(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            product_id,
            agent_cash,
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
            removal_reason,
        );

        assert_eq!(log.removal_reason, removal_reason);
        assert_eq!(log.agent_pref_original_price, None);
        assert_eq!(log.agent_pref_current_price, None);
    }

    #[test]
    fn test_generate_create_table_sql() {
        // 测试生成创建表的SQL语句
        let sql = generate_create_table_sql();

        // 验证SQL包含正确的表名
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS agent_demand_removal_logs"));

        // 验证SQL包含所有必要的字段
        assert!(sql.contains("id INT AUTO_INCREMENT PRIMARY KEY"));
        assert!(sql.contains("timestamp BIGINT NOT NULL"));
        assert!(sql.contains("round INT UNSIGNED NOT NULL"));
        assert!(sql.contains("task_id VARCHAR(255) NOT NULL"));
        assert!(sql.contains("agent_id INT UNSIGNED NOT NULL"));
        assert!(sql.contains("agent_name VARCHAR(255) NOT NULL"));
        assert!(sql.contains("product_id INT UNSIGNED NOT NULL"));
        assert!(sql.contains("agent_cash DOUBLE NOT NULL"));
        assert!(sql.contains("agent_pref_original_price DOUBLE"));
        assert!(sql.contains("agent_pref_original_elastic DOUBLE"));
        assert!(sql.contains("agent_pref_current_price DOUBLE"));
        assert!(sql.contains("agent_pref_current_range_lower DOUBLE"));
        assert!(sql.contains("agent_pref_current_range_upper DOUBLE"));
        assert!(sql.contains("removal_reason VARCHAR(255) NOT NULL"));
        assert!(sql.contains("created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"));

        // 验证SQL使用了正确的引擎和字符集
        assert!(sql.contains("ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"));
    }
}
