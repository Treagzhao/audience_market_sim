use crate::model::agent::preference::Preference;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

mod preference;

pub struct Agent {
    id: u64,
    name: String,
    preferences: Arc<RwLock<HashMap<u64, Preference>>>,
    cash: f64,
    demand: Arc<RwLock<HashMap<u64, bool>>>,
}

impl Agent {
    pub fn new(id: u64, name: String, cash: f64) -> Self {
        Agent {
            id,
            name,
            preferences: Arc::new(RwLock::new(HashMap::new())),
            cash,
            demand: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn preferences(&self) -> std::sync::RwLockReadGuard<'_, HashMap<u64, Preference>> {
        self.preferences.read().unwrap()
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    pub fn desire(&mut self) {
        let d = self.demand.clone();
        let p = self.preferences.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            loop {
                // 随机等待0~500ms
                let wait_time = rng.gen_range(0..501);
                thread::sleep(Duration::from_millis(wait_time));

                // 从preferences中随机选择一个商品ID
                let product_id = {
                    let preferences = p.read().unwrap();
                    if preferences.is_empty() {
                        continue; // 如果preferences为空，跳过本次循环
                    }

                    // 随机选择一个商品ID
                    let index = rng.gen_range(0..preferences.len());
                    *preferences.keys().nth(index).unwrap()
                };

                // 将商品需求加入demand映射，值为true
                {
                    let mut demand = d.write().unwrap();
                    demand.insert(product_id, true);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id = 1;
        let name = "test_agent".to_string();
        let cash = 100.0;

        let agent = Agent::new(id, name.clone(), cash);

        assert_eq!(agent.id(), id);
        assert_eq!(agent.name(), name);
        assert_eq!(agent.preferences().len(), 0); // 空map
        assert_eq!(agent.cash(), cash);
    }

    #[test]
    fn test_desire() {
        let id = 2;
        let name = "test_agent_desire".to_string();
        let cash = 100.0;

        let mut agent = Agent::new(id, name, cash);

        // 先添加一些preferences
        {
            let mut preferences = agent.preferences.write().unwrap();
            preferences.insert(1, Preference::new(10.0, 2.0));
            preferences.insert(2, Preference::new(20.0, 1.5));
            preferences.insert(3, Preference::new(30.0, 1.0));
        }

        // 调用desire方法启动需求生成线程
        agent.desire();

        // 等待1秒，让后台线程有机会生成一些需求
        thread::sleep(Duration::from_secs(1));

        // 检查demand映射是否有内容
        let demand = agent.demand.read().unwrap();
        assert!(
            demand.len() > 0,
            "Demand map should not be empty after calling desire"
        );

        // 获取preferences中的商品ID集合
        let valid_product_ids: std::collections::HashSet<u64> = {
            let preferences = agent.preferences.read().unwrap();
            preferences.keys().cloned().collect()
        };

        // 验证所有需求值都是true，且商品ID在preferences中存在
        for (product_id, is_demanded) in demand.iter() {
            assert!(
                *is_demanded,
                "Demand value for product {product_id} should be true"
            );
            assert!(
                valid_product_ids.contains(product_id),
                "Product ID {product_id} should be in preferences"
            );
        }
    }
}
