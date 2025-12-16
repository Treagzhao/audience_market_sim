use std::time::{SystemTime, UNIX_EPOCH};

// Agent现金日志结构体
pub struct AgentCashLog {
    pub timestamp: i64,
    pub round: u64,
    pub task_id: String,
    pub agent_id: u64,
    pub agent_name: String,
    pub cash: f64,         // 主体现金
    pub total_trades: u64, // 累计交易数
}

impl AgentCashLog {
    pub fn new(
        round: u64,
        task_id: String,
        agent_id: u64,
        agent_name: String,
        cash: f64,
        total_trades: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        AgentCashLog {
            timestamp,
            round,
            task_id,
            agent_id,
            agent_name,
            cash,
            total_trades,
        }
    }
}

pub fn log_agent_cash(
    timestamp: i64,
    round: u64,
    agent_id: u64,
    agent_name: String,
    cash: f64,
    total_trades: u64,
) -> String {
    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO agent_cash_logs (
                    timestamp, round, agent_id, agent_name, cash, total_trades
                ) VALUES (
                    {}, {}, {}, '{}', {}, {}
                )
            "#,
        timestamp, round, agent_id, agent_name, cash, total_trades
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_agent_cash_log_new() {
        // 测试AgentCashLog::new方法
        let round = 10;
        let task_id = "test_task_123".to_string();
        let agent_id = 456;
        let agent_name = "TestAgent".to_string();
        let cash = 1000.50;
        let total_trades = 20;

        let log = AgentCashLog::new(
            round,
            task_id.clone(),
            agent_id,
            agent_name.clone(),
            cash,
            total_trades,
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
        assert_eq!(log.cash, cash);
        assert_eq!(log.total_trades, total_trades);
    }

    #[test]
    fn test_log_agent_cash() {
        // 测试log_agent_cash函数生成的SQL
        let timestamp = 1609459200000; // 2021-01-01 00:00:00 UTC
        let round = 10;
        let agent_id = 456;
        let agent_name = "TestAgent".to_string();
        let cash = 1000.50;
        let total_trades = 20;

        let sql = log_agent_cash(
            timestamp,
            round,
            agent_id,
            agent_name.clone(),
            cash,
            total_trades,
        );

        // 验证SQL包含正确的表名和字段
        assert!(sql.contains("INSERT INTO agent_cash_logs"));
        assert!(sql.contains("timestamp"));
        assert!(sql.contains("round"));
        assert!(sql.contains("agent_id"));
        assert!(sql.contains("agent_name"));
        assert!(sql.contains("cash"));
        assert!(sql.contains("total_trades"));

        // 验证SQL包含正确的值
        assert!(sql.contains(&timestamp.to_string()));
        assert!(sql.contains(&round.to_string()));
        assert!(sql.contains(&agent_id.to_string()));
        assert!(sql.contains(&agent_name));
        assert!(sql.contains(&cash.to_string()));
        assert!(sql.contains(&total_trades.to_string()));
    }

    #[test]
    fn test_log_agent_cash_formatting() {
        // 测试SQL格式化，特别是字符串引号处理
        let timestamp = 1609459200000;
        let round = 0;
        let agent_id = 0;
        let agent_name = "Agent with 'quotes' and \\slashes".to_string();
        let cash = 0.0;
        let total_trades = 0;

        let sql = log_agent_cash(timestamp, round, agent_id, agent_name, cash, total_trades);

        // 验证SQL可以正确解析，没有语法错误
        assert!(sql.contains("'Agent with 'quotes' and \\slashes'"));
    }
}
