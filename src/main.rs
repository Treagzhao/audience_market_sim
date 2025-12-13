mod model;
mod entity;
mod logging;
mod util;

use rand::{distributions::Alphanumeric, Rng};
use std::fs::File;
use std::io::Read;
use toml::Value;
use crate::entity::normal_distribute::NormalDistribution;
use crate::logging::init_logger;

/// 从config.toml文件初始化产品列表
fn init_products() -> Vec<crate::model::product::Product> {
    // 读取config.toml文件
    let mut file = File::open("config.toml").expect("Failed to open config.toml");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read config.toml");
    
    // 解析toml
    let value = contents.parse::<Value>().expect("Failed to parse config.toml");
    
    // 提取products数组
    let products_array = value.get("products").and_then(Value::as_array).expect("Failed to get products array");
    
    // 转换为Product对象
    let mut products = Vec::new();
    
    for product_value in products_array {
        // 提取产品属性
        let id = product_value.get("id").and_then(Value::as_integer).expect("Failed to get product id") as u64;
        let name = product_value.get("name").and_then(Value::as_str).expect("Failed to get product name").to_string();
        let mean_price = product_value.get("mean_price").and_then(Value::as_float).expect("Failed to get mean_price");
        let std_dev_price = product_value.get("std_dev_price").and_then(Value::as_float).expect("Failed to get std_dev_price");
        let mean_elastic = product_value.get("mean_elastic").and_then(Value::as_float).expect("Failed to get mean_elastic");
        let std_dev_elastic = product_value.get("std_dev_elastic").and_then(Value::as_float).expect("Failed to get std_dev_elastic");
        let mean_product_cost = product_value.get("mean_product_cost").and_then(Value::as_float).expect("Failed to get mean_product_cost");
        let std_dev_product_cost = product_value.get("std_dev_product_cost").and_then(Value::as_float).expect("Failed to get std_dev_product_cost");
        
        // 创建价格分布
        let price_distribution = NormalDistribution::new(mean_price, id, format!("{}_price_dist", name), std_dev_price);
        
        // 创建弹性分布
        let elastic_distribution = NormalDistribution::new(mean_elastic, id, format!("{}_elastic_dist", name), std_dev_elastic);
        
        // 创建成本分布
        let product_cost_distribution = NormalDistribution::new(mean_product_cost, id, format!("{}_cost_dist", name), std_dev_product_cost);
        
        // 创建Product对象
        let product = crate::model::product::Product::from(id, name, price_distribution, elastic_distribution, product_cost_distribution);
        products.push(product);
    }
    
    products
}

fn main() {
    // 生成随机task_id
    let task_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    
    // 初始化日志记录器，传递task_id
    if let Err(e) = init_logger("trade_logs.csv", task_id.clone()) {
        eprintln!("Failed to initialize logger: {}", e);
        return;
    }
    
    println!("Initializing products from config.toml...");
    let products = init_products();
    println!("Successfully initialized {} products!", products.len());
    
    // 创建市场对象
    println!("Creating market...");
    let mut market = crate::model::market::Market::new(products);
    println!("Market created successfully!");
    
    // 运行市场模拟
    println!("Starting market simulation...");
    println!("Task ID: {}", task_id);
    println!("Pausing for 5 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(5));
    market.run();
    println!("Market simulation {:?} completed!", task_id);
}
