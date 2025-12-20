mod entity;
mod logging;
mod model;
mod util;
use crate::entity::normal_distribute::NormalDistribution;
use crate::logging::Logger;
use parking_lot::deadlock;
use rand::{Rng, distributions::Alphanumeric};
use std::fs::File;
use std::io::Read;
use std::thread;
use std::time::Duration;
use toml::Value;
use crate::model::product::ProductCategory;

/// 从config.toml文件初始化产品列表
fn init_products() -> Vec<crate::model::product::Product> {
    // 读取config.toml文件
    let mut file = File::open("config.toml").expect("Failed to open config.toml");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Failed to read config.toml");

    // 解析toml
    let value = contents
        .parse::<Value>()
        .expect("Failed to parse config.toml");

    // 提取products数组
    let products_array = value
        .get("products")
        .and_then(Value::as_array)
        .expect("Failed to get products array");

    // 转换为Product对象
    let mut products = Vec::new();

    for product_value in products_array {
        // 提取产品属性
        let id = product_value
            .get("id")
            .and_then(Value::as_integer)
            .expect("Failed to get product id") as u64;
        let name = product_value
            .get("name")
            .and_then(Value::as_str)
            .expect("Failed to get product name")
            .to_string();
        let mean_price = product_value
            .get("mean_price")
            .and_then(Value::as_float)
            .expect("Failed to get mean_price");
        let std_dev_price = product_value
            .get("std_dev_price")
            .and_then(Value::as_float)
            .expect("Failed to get std_dev_price");
        let mean_elastic = product_value
            .get("mean_elastic")
            .and_then(Value::as_float)
            .expect("Failed to get mean_elastic");
        let std_dev_elastic = product_value
            .get("std_dev_elastic")
            .and_then(Value::as_float)
            .expect("Failed to get std_dev_elastic");
        let mean_product_cost = product_value
            .get("mean_product_cost")
            .and_then(Value::as_float)
            .expect("Failed to get mean_product_cost");
        let std_dev_product_cost = product_value
            .get("std_dev_product_cost")
            .and_then(Value::as_float)
            .expect("Failed to get std_dev_product_cost");
        let product_category = product_value
            .get("category")
            .and_then(Value::as_str)
            .expect("Failed to get product_category")
            .to_string();

        // 创建价格分布
        let price_distribution = NormalDistribution::new(
            mean_price,
            id,
            format!("{}_price_dist", name),
            std_dev_price,
        );

        // 创建弹性分布
        let elastic_distribution = NormalDistribution::new(
            mean_elastic,
            id,
            format!("{}_elastic_dist", name),
            std_dev_elastic,
        );
        // 提取durability属性
        let durability = product_value
            .get("durability")
            .and_then(Value::as_float)
            .expect("Failed to get durability") as f64;

        // 创建成本分布
        let product_cost_distribution = NormalDistribution::new(
            mean_product_cost,
            id,
            format!("{}_cost_dist", name),
            std_dev_product_cost,
        );

        // 创建Product对象
        let product = crate::model::product::Product::from(
            id,
            name,
            ProductCategory::from_str(&product_category),
            durability,
            price_distribution,
            elastic_distribution,
            product_cost_distribution,
        );
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

    {
        let mut logger = logging::LOGGER.write();
        logger.set_task_id(task_id.clone());
        drop(logger);
    }
    // 启动一个线程定期检测死锁
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));
            let deadlocks = deadlock::check_deadlock();
            if !deadlocks.is_empty() {
                let deadlock_threads = &deadlocks[0];
                if deadlock_threads.len() > 0 {
                    let deadlock_thread = &deadlock_threads[0];
                    println!("检测到死锁! 线程 {:?} ", deadlock_thread.thread_id());
                    {
                        let backtrace = deadlock_thread.backtrace();
                        println!("死锁线程 {:?} 的调用栈:", deadlock_thread.thread_id());
                        println!("{:?}", backtrace);
                    }
                }
            }
        }
    });
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
