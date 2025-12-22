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
    pub initial_stock: u16,
    pub remaining_stock: u16,
    pub supply_range_lower: f64,
    pub supply_range_upper: f64,
    // 新增财务字段
    pub units_sold: u16,
    pub revenue: f64,
    pub total_stock: u16,
    pub total_production: u16,
    pub rot_stock: u16,
    pub production_cost: f64,
    pub profit: f64,
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
        initial_stock: u16,
        remaining_stock: u16,
        supply_range_lower: f64,
        supply_range_upper: f64,
        // 新增财务字段参数
        units_sold: u16,
        revenue: f64,
        total_stock: u16,
        total_production: u16,
        rot_stock: u16,
        production_cost: f64,
        profit: f64,
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
            // 新增财务字段赋值
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
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
        units_sold SMALLINT NOT NULL,
        revenue DOUBLE NOT NULL,
        total_stock SMALLINT NOT NULL,
        total_production SMALLINT NOT NULL,
        rot_stock SMALLINT NOT NULL,
        production_cost DOUBLE NOT NULL,
        profit DOUBLE NOT NULL,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
    "#
    .to_string()
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
    initial_stock: u16,
    remaining_stock: u16,
    supply_range_lower: f64,
    supply_range_upper: f64,
    // 新增财务字段参数
    units_sold: u16,
    revenue: f64,
    total_stock: u16,
    total_production: u16,
    rot_stock: u16,
    production_cost: f64,
    profit: f64,
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
        // 新增财务字段赋值
        units_sold,
        revenue,
        total_stock,
        total_production,
        rot_stock,
        production_cost,
        profit,
    );

    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO factory_end_of_round_logs (
                    timestamp, round, task_id, factory_id, factory_name, product_id, product_category,
                    cash, initial_stock, remaining_stock, supply_range_lower, supply_range_upper,
                    units_sold, revenue, total_stock, total_production, rot_stock, production_cost, profit
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {}, '{}',
                    {}, {}, {}, {}, {},
                    {}, {}, {}, {}, {}, {}, {}
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
        log.supply_range_upper,
        // 新增财务字段值
        log.units_sold,
        log.revenue,
        log.total_stock,
        log.total_production,
        log.rot_stock,
        log.production_cost,
        log.profit
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
        // 新增财务字段测试值
        let units_sold = 70;
        let revenue = 1500.0;
        let total_stock = 100;
        let total_production = 100;
        let rot_stock = 5;
        let production_cost = 800.0;
        let profit = 700.0;

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
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
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
        // 验证新增财务字段
        assert_eq!(log.units_sold, units_sold);
        assert_eq!(log.revenue, revenue);
        assert_eq!(log.total_stock, total_stock);
        assert_eq!(log.total_production, total_production);
        assert_eq!(log.rot_stock, rot_stock);
        assert_eq!(log.production_cost, production_cost);
        assert_eq!(log.profit, profit);
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
        // 新增财务字段测试值
        let units_sold = 0;
        let revenue = 0.0;
        let total_stock = 0;
        let total_production = 0;
        let rot_stock = 0;
        let production_cost = 0.0;
        let profit = 0.0;

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
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
        );

        assert_eq!(log.initial_stock, initial_stock);
        assert_eq!(log.remaining_stock, remaining_stock);
        assert_eq!(log.supply_range_lower, supply_range_lower);
        assert_eq!(log.supply_range_upper, supply_range_upper);
        // 验证新增财务字段
        assert_eq!(log.units_sold, units_sold);
        assert_eq!(log.revenue, revenue);
        assert_eq!(log.total_stock, total_stock);
        assert_eq!(log.total_production, total_production);
        assert_eq!(log.rot_stock, rot_stock);
        assert_eq!(log.production_cost, production_cost);
        assert_eq!(log.profit, profit);
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
        // 新增财务字段测试值
        let units_sold = 70;
        let revenue = 1500.0;
        let total_stock = 100;
        let total_production = 100;
        let rot_stock = 5;
        let production_cost = 800.0;
        let profit = 700.0;

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
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
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
        // 验证SQL包含新增的财务字段
        assert!(sql.contains("units_sold"));
        assert!(sql.contains("revenue"));
        assert!(sql.contains("total_stock"));
        assert!(sql.contains("total_production"));
        assert!(sql.contains("rot_stock"));
        assert!(sql.contains("production_cost"));
        assert!(sql.contains("profit"));

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
        // 验证SQL包含新增财务字段的值
        assert!(sql.contains(&units_sold.to_string()));
        assert!(sql.contains(&revenue.to_string()));
        assert!(sql.contains(&total_stock.to_string()));
        assert!(sql.contains(&total_production.to_string()));
        assert!(sql.contains(&rot_stock.to_string()));
        assert!(sql.contains(&production_cost.to_string()));
        assert!(sql.contains(&profit.to_string()));
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
        // 新增财务字段测试值
        let units_sold = 0; // 库存没有变化，销售量为0
        let revenue = 0.0;
        let total_stock = 100;
        let total_production = 0;
        let rot_stock = 0;
        let production_cost = 0.0;
        let profit = 0.0;

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
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
        );

        // 验证SQL生成正确
        assert!(sql.contains("INSERT INTO factory_end_of_round_logs"));
        assert!(sql.contains(&initial_stock.to_string()));
        assert!(sql.contains(&remaining_stock.to_string()));
        assert!(sql.contains(&"TestCategory"));
        assert!(sql.contains(&"TestCategory"));
        // 验证SQL包含新增财务字段
        assert!(sql.contains(&units_sold.to_string()));
        assert!(sql.contains(&revenue.to_string()));
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
        // 新增财务字段测试值
        let units_sold = 0;
        let revenue = 0.0;
        let total_stock = 0;
        let total_production = 0;
        let rot_stock = 0;
        let production_cost = 0.0;
        let profit = 0.0;

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
            units_sold,
            revenue,
            total_stock,
            total_production,
            rot_stock,
            production_cost,
            profit,
        );

        // 验证SQL生成正确
        assert!(sql.contains("INSERT INTO factory_end_of_round_logs"));
        assert!(sql.contains(&initial_stock.to_string()));
        assert!(sql.contains(&remaining_stock.to_string()));
        // 验证SQL包含新增财务字段
        assert!(sql.contains(&units_sold.to_string()));
        assert!(sql.contains(&revenue.to_string()));
    }
}
