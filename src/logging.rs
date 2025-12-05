use crate::model::agent::Agent;
use crate::model::agent::TradeResult;
use crate::model::factory::Factory;
use crate::model::product::Product;
use csv::Writer;
use std::fs::File;
use std::sync::{Arc, Mutex, RwLock};

// 交易日志结构体
pub struct TradeLog {
    round: u64,
    trade_id: u64,
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
    // Agent preference fields
    agent_pref_original_price: Option<f64>,
    agent_pref_original_elastic: Option<f64>,
    agent_pref_current_price: Option<f64>,
    agent_pref_current_range_lower: Option<f64>,
    agent_pref_current_range_upper: Option<f64>,
}

impl TradeLog {
    pub fn new(
        round: u64,
        trade_id: u64,
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
        // 获取agent对该产品的偏好
        let preferences = agent.preferences();
        let preference = preferences.get(&product.id());

        // 提取偏好信息
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

        TradeLog {
            round,
            trade_id,
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

// 日志记录器
pub struct Logger {
    csv_writer: Mutex<Writer<File>>,
    trade_counter: Mutex<u64>,
}

impl Logger {
    pub fn new(file_path: &str) -> Result<Self, csv::Error> {
        // 使用create会自动截断文件，清空原有内容
        let file = File::create(file_path)?;
        let mut csv_writer = Writer::from_writer(file);

        // 写入CSV头
        csv_writer.write_record([
            "round",
            "trade_id",
            "agent_id",
            "agent_name",
            "agent_cash",
            "factory_id",
            "factory_name",
            "product_id",
            "product_name",
            "trade_result",
            "price",
            "factory_supply_range_lower",
            "factory_supply_range_upper",
            "factory_stock",
            "agent_pref_original_price",
            "agent_pref_original_elastic",
            "agent_pref_current_price",
            "agent_pref_current_range_lower",
            "agent_pref_current_range_upper",
        ])?;

        Ok(Logger {
            csv_writer: Mutex::new(csv_writer),
            trade_counter: Mutex::new(0),
        })
    }

    pub fn log_trade(
        &self,
        round: u64,
        agent: Arc<RwLock<Agent>>,
        factory: &Factory,
        product: &Product,
        trade_result: &TradeResult,
    ) -> Result<(), csv::Error> {
        let mut counter = self.trade_counter.lock().unwrap();
        *counter += 1;
        let trade_id = *counter;

        let log = TradeLog::new(round, trade_id, agent, factory, product, trade_result);

        let mut writer = self.csv_writer.lock().unwrap();
        writer.write_record([
            log.round.to_string(),
            log.trade_id.to_string(),
            log.agent_id.to_string(),
            log.agent_name,
            log.agent_cash.to_string(),
            log.factory_id.to_string(),
            log.factory_name,
            log.product_id.to_string(),
            log.product_name,
            log.trade_result,
            log.price
                .map(|p| p.to_string())
                .unwrap_or("-1.0".to_string()),
            log.factory_supply_range_lower.to_string(),
            log.factory_supply_range_upper.to_string(),
            log.factory_stock.to_string(),
            log.agent_pref_original_price
                .map(|p| p.to_string())
                .unwrap_or("-1.0".to_string()),
            log.agent_pref_original_elastic
                .map(|e| e.to_string())
                .unwrap_or("-1.0".to_string()),
            log.agent_pref_current_price
                .map(|p| p.to_string())
                .unwrap_or("-1.0".to_string()),
            log.agent_pref_current_range_lower
                .map(|r| r.to_string())
                .unwrap_or("-1.0".to_string()),
            log.agent_pref_current_range_upper
                .map(|r| r.to_string())
                .unwrap_or("-1.0".to_string()),
        ])?;

        writer.flush()?;

        Ok(())
    }
}

// 全局日志记录器
lazy_static::lazy_static! {
    pub static ref LOGGER: Mutex<Option<Logger>> = Mutex::new(None);
}

// 初始化日志记录器
pub fn init_logger(file_path: &str) -> Result<(), csv::Error> {
    let logger = Logger::new(file_path)?;
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
) -> Result<(), csv::Error> {
    if let Some(logger) = &mut *LOGGER.lock().unwrap() {
        logger.log_trade(round, agent, factory, product, trade_result)
    } else {
        Err(csv::Error::from(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Logger not initialized",
        )))
    }
}
