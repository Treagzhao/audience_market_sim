// 工厂轮次结束日志结构体
pub struct FactoryEndOfRoundLog {
    pub timestamp: i64,
    pub round: u64,
    pub task_id: String,
    pub factory_id: u64,
    pub factory_name: String,
    pub product_id: u64,
    pub product_category: String,
    pub cash: f64,
    pub initial_stock: i16,
    pub remaining_stock: i16,
    pub supply_range_lower: f64,
    pub supply_range_upper: f64,
}

impl FactoryEndOfRoundLog {
    pub fn new(
        timestamp: i64,
        round: u64,
        task_id: String,
        factory_id: u64,
        factory_name: String,
        product_id: u64,
        product_category: String,
        cash: f64,
        initial_stock: i16,
        remaining_stock: i16,
        supply_range_lower: f64,
        supply_range_upper: f64,
    ) -> Self {
        FactoryEndOfRoundLog {
            timestamp,
            round,
            task_id,
            factory_id,
            factory_name,
            product_id,
            product_category,
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        }
    }
}

// 生成创建表的SQL语句
pub fn generate_create_table_sql() -> String {
    r#"
    CREATE TABLE IF NOT EXISTS factory_end_of_round_logs (
        id INT AUTO_INCREMENT PRIMARY KEY,
        timestamp BIGINT NOT NULL,
        round INT UNSIGNED NOT NULL,
        task_id VARCHAR(255) NOT NULL,
        factory_id INT UNSIGNED NOT NULL,
        factory_name VARCHAR(255) NOT NULL,
        product_id INT UNSIGNED NOT NULL,
        product_category VARCHAR(255) NOT NULL,
        cash DOUBLE NOT NULL,
        initial_stock SMALLINT NOT NULL,
        remaining_stock SMALLINT NOT NULL,
        supply_range_lower DOUBLE NOT NULL,
        supply_range_upper DOUBLE NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#.to_string()
}

pub fn log_factory_end_of_round(
    timestamp: i64,
    round: u64,
    task_id: String,
    factory_id: u64,
    factory_name: String,
    product_id: u64,
    product_category: String,
    cash: f64,
    initial_stock: i16,
    remaining_stock: i16,
    supply_range_lower: f64,
    supply_range_upper: f64,
) -> String {
    let log = FactoryEndOfRoundLog::new(
        timestamp,
        round,
        task_id.clone(),
        factory_id,
        factory_name.clone(),
        product_id,
        product_category,
        cash,
        initial_stock,
        remaining_stock,
        supply_range_lower,
        supply_range_upper,
    );

    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO factory_end_of_round_logs (
                    timestamp, round, task_id, factory_id, factory_name, product_id, product_category,
                    cash, initial_stock, remaining_stock, supply_range_lower, supply_range_upper
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {}, '{}',
                    {}, {}, {}, {}, {}
                )
            "#,
        log.timestamp,
        log.round,
        log.task_id,
        log.factory_id,
        log.factory_name,
        log.product_id,
        log.product_category,
        log.cash,
        log.initial_stock,
        log.remaining_stock,
        log.supply_range_lower,
        log.supply_range_upper
    );
    sql
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_end_of_round_log_new() {
        // 测试FactoryEndOfRoundLog::new方法
        let timestamp = 1609459200000; // 2021-01-01 00:00:00 UTC
        let round = 25;
        let task_id = "test_task_321".to_string();
        let factory_id = 654;
        let factory_name = "TestFactory".to_string();
        let product_id = 303;
        let cash = 2000.75;
        let initial_stock = 100;
        let remaining_stock = 30;
        let supply_range_lower = 50.0;
        let supply_range_upper = 150.0;

        let log = FactoryEndOfRoundLog::new(
            timestamp,
            round,
            task_id.clone(),
            factory_id,
            factory_name.clone(),
            product_id,
            "TestCategory".to_string(),
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );

        // 验证所有字段
        assert_eq!(log.timestamp, timestamp);
        assert_eq!(log.round, round);
        assert_eq!(log.task_id, task_id);
        assert_eq!(log.factory_id, factory_id);
        assert_eq!(log.factory_name, factory_name);
        assert_eq!(log.product_id, product_id);
        assert_eq!(log.product_category, "TestCategory");
        assert_eq!(log.cash, cash);
        assert_eq!(log.initial_stock, initial_stock);
        assert_eq!(log.remaining_stock, remaining_stock);
        assert_eq!(log.supply_range_lower, supply_range_lower);
        assert_eq!(log.supply_range_upper, supply_range_upper);
    }

    #[test]
    fn test_factory_end_of_round_log_new_with_zero_stock() {
        // 测试库存为0的情况
        let timestamp = 1609459200000;
        let round = 25;
        let task_id = "test_task_321".to_string();
        let factory_id = 654;
        let factory_name = "TestFactory".to_string();
        let product_id = 303;
        let cash = 2000.75;
        let initial_stock = 0;
        let remaining_stock = 0;
        let supply_range_lower = 50.0;
        let supply_range_upper = 150.0;

        let log = FactoryEndOfRoundLog::new(
            timestamp,
            round,
            task_id.clone(),
            factory_id,
            factory_name.clone(),
            product_id,
            "TestCategory".to_string(),
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );

        assert_eq!(log.initial_stock, initial_stock);
        assert_eq!(log.remaining_stock, remaining_stock);
        assert_eq!(log.supply_range_lower, supply_range_lower);
        assert_eq!(log.supply_range_upper, supply_range_upper);
    }

    #[test]
    fn test_generate_create_table_sql() {
        // 测试生成创建表的SQL语句
        let sql = generate_create_table_sql();

        // 验证SQL包含正确的表名
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS factory_end_of_round_logs"));
        
        // 验证SQL包含所有必要的字段
        assert!(sql.contains("id INT AUTO_INCREMENT PRIMARY KEY"));
        assert!(sql.contains("timestamp BIGINT NOT NULL"));
        assert!(sql.contains("round INT UNSIGNED NOT NULL"));
        assert!(sql.contains("task_id VARCHAR(255) NOT NULL"));
        assert!(sql.contains("factory_id INT UNSIGNED NOT NULL"));
        assert!(sql.contains("factory_name VARCHAR(255) NOT NULL"));
        assert!(sql.contains("product_id INT UNSIGNED NOT NULL"));
        assert!(sql.contains("product_category VARCHAR(255) NOT NULL"));
        assert!(sql.contains("cash DOUBLE NOT NULL"));
        assert!(sql.contains("initial_stock SMALLINT NOT NULL"));
        assert!(sql.contains("remaining_stock SMALLINT NOT NULL"));
        assert!(sql.contains("supply_range_lower DOUBLE NOT NULL"));
        assert!(sql.contains("supply_range_upper DOUBLE NOT NULL"));
        assert!(sql.contains("created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP"));
        
        // 验证SQL使用了正确的引擎和字符集
        assert!(sql.contains("ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"));
    }

    #[test]
    fn test_log_factory_end_of_round() {
        // 测试log_factory_end_of_round函数生成的SQL
        let timestamp = 1609459200000; // 2021-01-01 00:00:00 UTC
        let round = 25;
        let task_id = "test_task_321".to_string();
        let factory_id = 654;
        let factory_name = "TestFactory".to_string();
        let product_id = 303;
        let cash = 2000.75;
        let initial_stock = 100;
        let remaining_stock = 30;
        let supply_range_lower = 50.0;
        let supply_range_upper = 150.0;

        let sql = log_factory_end_of_round(
            timestamp,
            round,
            task_id.clone(),
            factory_id,
            factory_name.clone(),
            product_id,
            "TestCategory".to_string(),
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );

        // 验证SQL包含正确的表名和字段
        assert!(sql.contains("INSERT INTO factory_end_of_round_logs"));
        assert!(sql.contains("timestamp"));
        assert!(sql.contains("round"));
        assert!(sql.contains("task_id"));
        assert!(sql.contains("factory_id"));
        assert!(sql.contains("factory_name"));
        assert!(sql.contains("product_id"));
        assert!(sql.contains("product_category"));
        assert!(sql.contains("cash"));
        assert!(sql.contains("initial_stock"));
        assert!(sql.contains("remaining_stock"));
        assert!(sql.contains("supply_range_lower"));
        assert!(sql.contains("supply_range_upper"));

        // 验证SQL包含正确的值
        assert!(sql.contains(&timestamp.to_string()));
        assert!(sql.contains(&round.to_string()));
        assert!(sql.contains(&task_id));
        assert!(sql.contains(&factory_id.to_string()));
        assert!(sql.contains(&factory_name));
        assert!(sql.contains(&product_id.to_string()));
        assert!(sql.contains(&"TestCategory"));
        assert!(sql.contains(&cash.to_string()));
        assert!(sql.contains(&initial_stock.to_string()));
        assert!(sql.contains(&remaining_stock.to_string()));
        assert!(sql.contains(&supply_range_lower.to_string()));
        assert!(sql.contains(&supply_range_upper.to_string()));
    }

    #[test]
    fn test_log_factory_end_of_round_with_no_stock_change() {
        // 测试库存没有变化的情况
        let timestamp = 1609459200000;
        let round = 25;
        let task_id = "test_task_321".to_string();
        let factory_id = 654;
        let factory_name = "TestFactory".to_string();
        let product_id = 303;
        let cash = 2000.75;
        let initial_stock = 100;
        let remaining_stock = 100; // 库存没有变化
        let supply_range_lower = 50.0;
        let supply_range_upper = 150.0;

        let sql = log_factory_end_of_round(
            timestamp,
            round,
            task_id.clone(),
            factory_id,
            factory_name.clone(),
            product_id,
            "TestCategory".to_string(),
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );

        // 验证SQL生成正确
        assert!(sql.contains("INSERT INTO factory_end_of_round_logs"));
        assert!(sql.contains(&initial_stock.to_string()));
        assert!(sql.contains(&remaining_stock.to_string()));
        assert!(sql.contains(&"TestCategory"));
        assert!(sql.contains(&"TestCategory"));
    }

    #[test]
    fn test_log_factory_end_of_round_with_zero_stock() {
        // 测试库存为0的情况
        let timestamp = 1609459200000;
        let round = 25;
        let task_id = "test_task_321".to_string();
        let factory_id = 654;
        let factory_name = "TestFactory".to_string();
        let product_id = 303;
        let cash = 2000.75;
        let initial_stock = 0;
        let remaining_stock = 0;
        let supply_range_lower = 50.0;
        let supply_range_upper = 150.0;

        let sql = log_factory_end_of_round(
            timestamp,
            round,
            task_id.clone(),
            factory_id,
            factory_name.clone(),
            product_id,
            "TestCategory".to_string(),
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );

        // 验证SQL生成正确
        assert!(sql.contains("INSERT INTO factory_end_of_round_logs"));
        assert!(sql.contains(&initial_stock.to_string()));
        assert!(sql.contains(&remaining_stock.to_string()));
    }
}
