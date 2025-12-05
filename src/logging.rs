use crate::model::agent::Agent;
use crate::model::agent::TradeResult;
use crate::model::factory::Factory;
use crate::model::product::Product;
use lazy_static::lazy_static;
use mysql::prelude::{FromRow, Queryable};
use mysql::{OptsBuilder, Pool};
use std::env;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

// 初始化MySQL连接池
lazy_static! {
    pub static ref MYSQL_POOL: OnceLock<Pool> = OnceLock::new();
}

// 初始化MySQL连接池
pub fn init_mysql_client() {
    let host = env::var("MYSQL_HOST").unwrap_or("localhost".to_string());
    let port = env::var("MYSQL_PORT").unwrap_or("3306".to_string());
    let user = env::var("MYSQL_USER").unwrap_or("root".to_string());
    let password = env::var("MYSQL_PASSWORD").unwrap_or("".to_string());
    let database = env::var("MYSQL_DATABASE").unwrap_or("austrian_market".to_string());

    println!(
        "Initializing MySQL client with host: {}, port: {}, user: {}, database: {}",
        host, port, user, database
    );

    // 使用OptsBuilder创建连接选项
    let opts = OptsBuilder::new()
        .ip_or_hostname(Some(host))
        .tcp_port(port.parse::<u16>().unwrap_or(3306))
        .user(Some(user))
        .pass(Some(password))
        .db_name(Some(database));

    match Pool::new(opts) {
        Ok(pool) => {
            // 保存到全局静态变量
            match MYSQL_POOL.set(pool) {
                Ok(_) => {
                    println!("MySQL pool initialized successfully");
                }
                Err(_) => {
                    eprintln!("Failed to initialize MySQL pool: Pool already exists");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to create MySQL pool: {}", e);
        }
    }
}

// 交易日志结构体
pub struct TradeLog {
    timestamp: i64,
    round: u64,
    trade_id: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    agent_cash: f64,
    factory_id: u64,
    factory_name: String,
    product_id: u64,
    product_name: String,
    trade_result: String,
    price: Option<f64>,
    factory_supply_range_lower: f64,
    factory_supply_range_upper: f64,
    factory_stock: i16,
    agent_pref_original_price: Option<f64>,
    agent_pref_original_elastic: Option<f64>,
    agent_pref_current_price: Option<f64>,
    agent_pref_current_range_lower: Option<f64>,
    agent_pref_current_range_upper: Option<f64>,
}

// 工厂范围优化日志结构体
pub struct FactoryRangeOptimizationLog {
    timestamp: i64,
    round: u64,
    task_id: String,
    factory_id: u64,
    factory_name: String,
    product_id: u64,
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

// Agent范围调整日志结构体
pub struct AgentRangeAdjustmentLog {
    timestamp: i64,
    round: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    product_id: u64,
    old_range_lower: f64,
    old_range_upper: f64,
    new_range_lower: f64,
    new_range_upper: f64,
    lower_change: f64,
    upper_change: f64,
    min_change_ratio: f64,
    max_change_ratio: f64,
    center: f64,
    adjustment_type: String, // "trade_success" 或 "trade_failed"
    price: Option<f64>,      // 仅在交易成功时有值
}

// Agent现金日志结构体
pub struct AgentCashLog {
    timestamp: i64,
    round: u64,
    task_id: String,
    agent_id: u64,
    agent_name: String,
    cash: f64,         // 主体现金
    total_trades: u64, // 累计交易数
}

impl TradeLog {
    pub fn new(
        round: u64,
        trade_id: u64,
        task_id: String,
        a: Arc<RwLock<Agent>>,
        factory: &Factory,
        product: &Product,
        trade_result: &TradeResult,
    ) -> Self {
        let (result_str, price) = match trade_result {
            TradeResult::NotMatched => ("NotMatched", None),
            TradeResult::Failed => ("Failed", None),
            TradeResult::Success(p) => ("Success", Some(*p)),
            TradeResult::NotYet => ("NotYet", None),
        };

        let (lower, upper) = factory.supply_price_range();
        let agent = a.read().unwrap();
        let preferences = agent.preferences();
        let preference = preferences.get(&product.id());

        let (
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
        ) = match preference {
            Some(pref) => (
                Some(pref.original_price),
                Some(pref.original_elastic),
                Some(pref.current_price),
                Some(pref.current_range.0),
                Some(pref.current_range.1),
            ),
            None => (None, None, None, None, None),
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Failed to get system time")
            .as_millis() as i64;

        TradeLog {
            timestamp,
            round,
            trade_id,
            task_id,
            agent_id: agent.id(),
            agent_name: agent.name().to_string(),
            agent_cash: agent.cash(),
            factory_id: factory.id(),
            factory_name: factory.name().to_string(),
            product_id: product.id(),
            product_name: product.name().to_string(),
            trade_result: result_str.to_string(),
            price,
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

impl FactoryRangeOptimizationLog {
    pub fn new(
        round: u64,
        task_id: String,
        factory_id: u64,
        factory_name: String,
        product_id: u64,
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

impl AgentCashLog {
    pub fn new(
        timestamp: i64,
        round: u64,
        task_id: String,
        agent_id: u64,
        agent_name: String,
        cash: f64,
        total_trades: u64,
    ) -> Self {
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

// 日志记录器
#[derive(Clone)]
pub struct Logger {
    trade_counter: Arc<Mutex<u64>>,
    task_id: String,
}

impl Logger {
    pub fn new(_file_path: &str, task_id: String) -> Result<Self, Box<dyn std::error::Error>> {
        init_mysql_client();

        Ok(Logger {
            trade_counter: Arc::new(Mutex::new(0)),
            task_id,
        })
    }

    pub fn log_trade(
        &self,
        round: u64,
        agent: Arc<RwLock<Agent>>,
        factory: &Factory,
        product: &Product,
        trade_result: &TradeResult,
        trade_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let log = TradeLog::new(
            round,
            trade_id,
            self.task_id.clone(),
            agent,
            factory,
            product,
            trade_result,
        );

        // 如果MySQL池未初始化，直接返回成功
        let Some(pool) = MYSQL_POOL.get() else {
            return Ok(());
        };

        // 准备SQL语句
        let sql = r#"
            INSERT INTO trade_logs (
                timestamp, round, trade_id, task_id, agent_id, agent_name, agent_cash,
                factory_id, factory_name, product_id, product_name, trade_result, price,
                factory_supply_range_lower, factory_supply_range_upper, factory_stock,
                agent_pref_original_price, agent_pref_original_elastic, agent_pref_current_price,
                agent_pref_current_range_lower, agent_pref_current_range_upper
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?,
                ?, ?, ?, ?, ?, ?,
                ?, ?, ?,
                ?, ?, ?,
                ?, ?
            )
        "#;

        // 获取连接并执行SQL插入
        let mut conn = pool.get_conn()?;
        // 使用exec方法，MySQL的exec方法不支持超过20个参数的元组，所以我们使用字符串格式化来构建SQL
        let sql = format!(
            r#"
                INSERT INTO trade_logs (
                    timestamp, round, trade_id, task_id, agent_id, agent_name, agent_cash,
                    factory_id, factory_name, product_id, product_name, trade_result, price,
                    factory_supply_range_lower, factory_supply_range_upper, factory_stock,
                    agent_pref_original_price, agent_pref_original_elastic, agent_pref_current_price,
                    agent_pref_current_range_lower, agent_pref_current_range_upper
                ) VALUES (
                    {}, {}, {}, '{}', {}, '{}', {},
                    {}, '{}', {}, '{}', '{}', {},
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
            log.price.unwrap_or(-1.0),
            log.factory_supply_range_lower,
            log.factory_supply_range_upper,
            log.factory_stock,
            log.agent_pref_original_price.unwrap_or(-1.0),
            log.agent_pref_original_elastic.unwrap_or(-1.0),
            log.agent_pref_current_price.unwrap_or(-1.0),
            log.agent_pref_current_range_lower.unwrap_or(-1.0),
            log.agent_pref_current_range_upper.unwrap_or(-1.0),
        );

        // 使用query方法执行SQL
        conn.query_drop(&sql)?;

        Ok(())
    }

    pub fn log_factory_range_optimization(
        &self,
        round: u64,
        factory_id: u64,
        factory_name: String,
        product_id: u64,
        old_range: (f64, f64),
        new_range: (f64, f64),
        lower_change: f64,
        upper_change: f64,
        total_change: f64,
        lower_change_ratio: f64,
        upper_change_ratio: f64,
        trade_result: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let log = FactoryRangeOptimizationLog::new(
            round,
            self.task_id.clone(),
            factory_id,
            factory_name,
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            total_change,
            lower_change_ratio,
            upper_change_ratio,
            trade_result,
        );

        // 如果MySQL池未初始化，直接返回成功
        let Some(pool) = MYSQL_POOL.get() else {
            return Ok(());
        };

        // 准备SQL语句
        let sql = format!(
            r#"
                INSERT INTO factory_range_optimization_logs (
                    timestamp, round, task_id, factory_id, factory_name, product_id,
                    old_range_lower, old_range_upper, new_range_lower, new_range_upper,
                    lower_change, upper_change, total_change,
                    lower_change_ratio, upper_change_ratio, trade_result
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {},
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

        // 使用query方法执行SQL
        let mut conn = pool.get_conn()?;
        conn.query_drop(&sql)?;

        Ok(())
    }

    pub fn log_agent_range_adjustment(
        &self,
        round: u64,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let log = AgentRangeAdjustmentLog::new(
            round,
            self.task_id.clone(),
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

        // 如果MySQL池未初始化，直接返回成功
        let Some(pool) = MYSQL_POOL.get() else {
            return Ok(());
        };

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
            log.price.unwrap_or(-1.0) // -1.0表示未设置
        );

        // 使用query方法执行SQL
        let mut conn = pool.get_conn()?;
        conn.query_drop(&sql)?;

        Ok(())
    }

    pub fn log_agent_cash(
        &self,
        timestamp: i64,
        round: u64,
        agent_id: u64,
        agent_name: String,
        cash: f64,
        total_trades: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let log = AgentCashLog::new(
            timestamp,
            round,
            self.task_id.clone(),
            agent_id,
            agent_name,
            cash,
            total_trades,
        );

        // 如果MySQL池未初始化，直接返回成功
        let Some(pool) = MYSQL_POOL.get() else {
            return Ok(());
        };

        // 准备SQL语句
        let sql = format!(
            r#"
                INSERT INTO agent_cash_logs (
                    timestamp, round, task_id, agent_id, agent_name, cash, total_trades
                ) VALUES (
                    {}, {}, '{}', {}, '{}', {}, {}
                )
            "#,
            log.timestamp,
            log.round,
            log.task_id,
            log.agent_id,
            log.agent_name,
            log.cash,
            log.total_trades
        );

        // 使用query方法执行SQL
        let mut conn = pool.get_conn()?;
        conn.query_drop(&sql)?;

        Ok(())
    }
}

// 全局日志记录器
lazy_static! {
    pub static ref LOGGER: Mutex<Option<Logger>> = Mutex::new(None);
}

// 初始化日志记录器
pub fn init_logger(file_path: &str, task_id: String) -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new(file_path, task_id)?;
    *LOGGER.lock().unwrap() = Some(logger);
    Ok(())
}

// 记录交易日志
pub fn log_trade(
    round: u64,
    agent: Arc<RwLock<Agent>>,
    factory: &Factory,
    product: &Product,
    trade_result: &TradeResult,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(logger) = &mut *LOGGER.lock().unwrap() {
        // 生成trade_id
        let mut counter = logger.trade_counter.lock().unwrap();
        *counter += 1;
        let trade_id = *counter;

        // 调用logger的log_trade方法
        if let Err(e) = logger.log_trade(round, agent, factory, product, trade_result, trade_id) {
            eprintln!("Failed to log trade to MySQL: {}", e);
        }
    }
    Ok(())
}

// 记录工厂范围优化日志
pub fn log_factory_range_optimization(
    round: u64,
    factory_id: u64,
    factory_name: String,
    product_id: u64,
    old_range: (f64, f64),
    new_range: (f64, f64),
    lower_change: f64,
    upper_change: f64,
    total_change: f64,
    lower_change_ratio: f64,
    upper_change_ratio: f64,
    trade_result: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(logger) = &mut *LOGGER.lock().unwrap() {
        // 调用logger的log_factory_range_optimization方法
        if let Err(e) = logger.log_factory_range_optimization(
            round,
            factory_id,
            factory_name,
            product_id,
            old_range,
            new_range,
            lower_change,
            upper_change,
            total_change,
            lower_change_ratio,
            upper_change_ratio,
            trade_result,
        ) {
            eprintln!("Failed to log factory range optimization to MySQL: {}", e);
        }
    }
    Ok(())
}

// 记录Agent范围调整日志
pub fn log_agent_range_adjustment(
    round: u64,
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
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(logger) = &mut *LOGGER.lock().unwrap() {
        // 调用logger的log_agent_range_adjustment方法
        if let Err(e) = logger.log_agent_range_adjustment(
            round,
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
        ) {
            eprintln!("Failed to log agent range adjustment to MySQL: {}", e);
        }
    }
    Ok(())
}

// 记录Agent现金日志
pub fn log_agent_cash(
    timestamp: i64,
    round: u64,
    agent_id: u64,
    agent_name: String,
    cash: f64,
    total_trades: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(logger) = &mut *LOGGER.lock().unwrap() {
        // 调用logger的log_agent_cash方法
        if let Err(e) =
            logger.log_agent_cash(timestamp, round, agent_id, agent_name, cash, total_trades)
        {
            eprintln!("Failed to log agent cash to MySQL: {}", e);
        }
    }
    Ok(())
}
