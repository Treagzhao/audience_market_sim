use std::time::{SystemTime, UNIX_EPOCH};
use crate::logging::MYSQL_POOL;

// 工厂范围优化日志结构体
pub struct FactoryRangeOptimizationLog {
    timestamp: i64,
    round: u64,
    task_id: String,
    factory_id: u64,
    factory_name: String,
    product_id: u64,
    product_category: String,
    old_range_lower: f64,
    old_range_upper: f64,
    new_range_lower: f64,
    new_range_upper: f64,
    lower_change: f64,
    upper_change: f64,
    total_change: f64,
    lower_change_ratio: f64,
    upper_change_ratio: f64,
    trade_result: String,
}

impl FactoryRangeOptimizationLog {
    pub fn new(
        round: u64,
        task_id: String,
        factory_id: u64,
        factory_name: String,
        product_id: u64,
        product_category: String,
        old_range: (f64, f64),
        new_range: (f64, f64),
        lower_change: f64,
        upper_change: f64,
        total_change: f64,
        lower_change_ratio: f64,
        upper_change_ratio: f64,
        trade_result: &str,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        FactoryRangeOptimizationLog {
            timestamp,
            round,
            task_id,
            factory_id,
            factory_name,
            product_id,
            product_category,
            old_range_lower: old_range.0,
            old_range_upper: old_range.1,
            new_range_lower: new_range.0,
            new_range_upper: new_range.1,
            lower_change,
            upper_change,
            total_change,
            lower_change_ratio,
            upper_change_ratio,
            trade_result: trade_result.to_string(),
        }
    }
}

pub fn log_factory_range_optimization( round: u64,
                                       task_id:String,
                                       factory_id: u64,
                                       factory_name: String,
                                       product_id: u64,
                                       product_category: String,
                                       old_range: (f64, f64),
                                       new_range: (f64, f64),
                                       lower_change: f64,
                                       upper_change: f64,
                                       total_change: f64,
                                       lower_change_ratio: f64,
                                       upper_change_ratio: f64,
                                       trade_result: &str,) -> String{
    let log = FactoryRangeOptimizationLog::new(
        round,
        task_id.clone(),
        factory_id,
        factory_name,
        product_id,
        product_category,
        old_range,
        new_range,
        lower_change,
        upper_change,
        total_change,
        lower_change_ratio,
        upper_change_ratio,
        trade_result,
    );


    // 准备SQL语句
    let sql = format!(
        r#"
                INSERT INTO factory_range_optimization_logs (
                    timestamp, round, task_id, factory_id, factory_name, product_id, product_category,
                    old_range_lower, old_range_upper, new_range_lower, new_range_upper,
                    lower_change, upper_change, total_change,
                    lower_change_ratio, upper_change_ratio, trade_result
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {}, '{}',
                    {}, {}, {}, {},
                    {}, {}, {},
                    {}, {}, '{}'
                )
            "#,
        log.timestamp,
        log.round,
        log.task_id,
        log.factory_id,
        log.factory_name,
        log.product_id,
        log.product_category,
        log.old_range_lower,
        log.old_range_upper,
        log.new_range_lower,
        log.new_range_upper,
        log.lower_change,
        log.upper_change,
        log.total_change,
        log.lower_change_ratio * 100.0, // 转换为百分比
        log.upper_change_ratio * 100.0, // 转换为百分比
        log.trade_result
    );
    sql
}