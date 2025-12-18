mod agent_cash_log;
mod agent_demand_removal_log;
mod agent_range_adjustment_log;
mod factory_end_of_round_log;
mod factory_range_optimization_log;
mod trade_log;

// 导入日志结构体和函数
use crate::logging::agent_cash_log::{AgentCashLog, log_agent_cash};
use crate::logging::agent_demand_removal_log::{log_agent_demand_removal, AgentDemandRemovalLog};
use crate::logging::agent_range_adjustment_log::{
    AgentRangeAdjustmentLog, log_agent_range_adjustment,
};
use crate::logging::factory_end_of_round_log::{log_factory_end_of_round, FactoryEndOfRoundLog};
use crate::logging::factory_range_optimization_log::FactoryRangeOptimizationLog;
pub use crate::logging::factory_range_optimization_log::log_factory_range_optimization;
use crate::logging::trade_log::{TradeLog, log_trade};
use crate::model::agent::Agent;
use crate::model::agent::TradeResult;
use crate::model::factory::Factory;
use crate::model::product::Product;
use lazy_static::lazy_static;
use mysql::prelude::{FromRow, Queryable};
use mysql::{OptsBuilder, Pool};
use parking_lot::{Mutex, RwLock};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, thread};

// 初始化MySQL连接池
lazy_static! {
    pub static ref MYSQL_POOL: OnceLock<Pool> = OnceLock::new();
    pub static ref LOGGER: Arc<RwLock<Logger>> = Arc::new(RwLock::new(
        Logger::new("trade_logs.csv", "".to_string()).unwrap()
    ));
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

// 日志记录器
pub struct Logger {
    task_id: String,
    tx: SyncSender<String>,
}

impl Logger {
    pub fn new(_file_path: &str, task_id: String) -> Result<Self, Box<dyn std::error::Error>> {
        init_mysql_client();
        let (tx, rx) = mpsc::sync_channel::<String>(30);
        thread::spawn(move || {
            let pool = MYSQL_POOL.get().unwrap();
            let mut conn = pool.get_conn().expect("Failed to get connection from pool");

            for sql in rx {
                let res = conn.query_drop(&sql);
                if let Err(e) = res {
                    eprintln!("Error executing SQL: {}", e);
                }
            }
        });
        Ok(Logger { task_id, tx })
    }

    pub fn set_task_id(&mut self, task_id: String) {
        self.task_id = task_id;
    }

    pub fn log_trade(
        &mut self,
        timestamp:i64,
        round: u64,
        trade_id: u64,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = log_trade(
            timestamp,
            round,
            trade_id,
            self.task_id.clone(),
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
        self.tx.send(sql)?;
        Ok(())
    }

    pub fn log_factory_range_optimization(
        &mut self,
        round: u64,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = log_factory_range_optimization(
            round,
            self.task_id.clone(),
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
        self.tx.send(sql)?;
        Ok(())
    }

    pub fn log_agent_range_adjustment(
        &mut self,
        round: u64,
        agent_id: u64,
        agent_name: String,
        product_id: u64,
        product_category: String,
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
        let sql = log_agent_range_adjustment(
            round,
            self.task_id.clone(),
            agent_id,
            agent_name,
            product_id,
            product_category,
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
        self.tx.send(sql)?;
        Ok(())
    }

    pub fn log_agent_cash(
        &mut self,
        timestamp: i64,
        round: u64,
        agent_id: u64,
        agent_name: String,
        cash: f64,
        total_trades: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = log_agent_cash(timestamp, self.task_id.clone(), round, agent_id, agent_name, cash, total_trades);
        self.tx.send(sql)?;
        Ok(())
    }

    pub fn log_agent_demand_removal(
        &self,
        round: u64,
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = log_agent_demand_removal(
            round,
            self.task_id.clone(),
            agent_id,
            agent_name,
            product_id,
            agent_cash,
            agent_pref_original_price,
            agent_pref_original_elastic,
            agent_pref_current_price,
            agent_pref_current_range_lower,
            agent_pref_current_range_upper,
            removal_reason,
        );
        self.tx.send(sql)?;
        Ok(())
    }

    pub fn log_factory_end_of_round(
        &self,
        timestamp: i64,
        round: u64,
        factory_id: u64,
        factory_name: String,
        product_id: u64,
        product_category: String,
        cash: f64,
        initial_stock: i16,
        remaining_stock: i16,
        supply_range_lower: f64,
        supply_range_upper: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sql = log_factory_end_of_round(
            timestamp,
            round,
            self.task_id.clone(),
            factory_id,
            factory_name,
            product_id,
            product_category,
            cash,
            initial_stock,
            remaining_stock,
            supply_range_lower,
            supply_range_upper,
        );
        self.tx.send(sql)?;
        Ok(())
    }
}
