use crate::model::agent::preference::Preference;
use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use crate::model::factory::Factory;
use crate::model::util::interval_intersection;

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

     fn match_factory(&self,factory:&Factory) -> Option<(f64,f64)> {
        let product_id = factory.product_id();
        let g = self.demand.read().unwrap();
        if !g.contains_key(&product_id) {
            return None;
        }
        let pg = self.preferences.read().unwrap();
        let p = pg.get(&product_id).unwrap();
        interval_intersection(p.current_range,factory.supply_price_range())
    }

    /// 处理交易失败的逻辑
    fn handle_trade_failure(&mut self, factory: &Factory, product_id: u64) {
        // 根据1-preference.elastic的概率决定是否删除demand
        let mut rng = rand::thread_rng();
        
        let mut g = self.preferences.write().unwrap();
        if let Some(preference) = g.get_mut(&product_id) {
            // 计算概率：弹性值本身，弹性越大，越容易删除需求
            let delete_probability = preference.original_elastic;
            
            // 生成随机数（0.0到1.0）
            let random_value = rng.gen_range(0.0..1.0);
            
            if random_value < delete_probability {
                // 删除demand
                if let Ok(mut demand) = self.demand.write() {
                    demand.remove(&product_id);
                }
            } else {
                // 不删除demand，更新range
                // 以factory的supply_price_range的lower值为中心
                let center = factory.supply_price_range().0;
                
                // 当前range长度
                let (min, max) = preference.current_range;
                let old_length = max - min;
                let new_length = old_length * 1.1; // 范围增大10%
                let half_new_length = new_length / 2.0;
                
                // 计算新的range
                let new_min = center - half_new_length;
                let new_max = center + half_new_length;
                
                // 确保最小值不小于0
                let new_min = new_min.max(0.0);
                
                // 更新range
                preference.current_range = (new_min, new_max);
            }
        }
    }
    
    pub fn trade(&mut self, factory:&Factory) -> Option<f64> {
        let g = self.preferences.write().unwrap();
        if !g.contains_key(&factory.product_id()) {
            return None;
        }
        drop(g);
        let merge_range = self.match_factory(factory);

        let product_id = factory.product_id();
        if let Some(range) = merge_range {
            // 根据range生成一个随机price值
            let (min_price, max_price) = range;
            let mut rng = rand::thread_rng();
            let mut price = rng.gen_range(min_price..max_price);
            
            // 检查price是否大于cash
            if price > self.cash {
                // 如果cash在交集范围内，就用cash作为price
                if self.cash >= min_price && self.cash <= max_price {
                    price = self.cash;
                } else {
                    // cash不在交集范围内，处理交易失败
                    self.handle_trade_failure(factory, product_id);
                    return None;
                }
            }

            let mut g = self.preferences.write().unwrap();
            let mut demand = self.demand.write().unwrap();
            demand.remove(&product_id);
            self.cash -= price;
            let preference = g.get_mut(&product_id).unwrap();
            preference.current_price = price;
            
            // 更新current_range，以新price为中点，范围比之前小10%
            let (mut min, mut max) = preference.current_range;
            let old_length = max - min;
            let new_length = old_length * 0.9; // 范围缩小10%
            let half_new_length = new_length / 2.0;
            
            // 计算新的范围，确保不小于0
            min = price - half_new_length;
            max = price + half_new_length;
            min = min.max(0.0);
            
            preference.current_range = (min, max);
            return Some(price);
        }else{
            // 没有交集，处理交易失败
            self.handle_trade_failure(factory, product_id);
        }
        return None;
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
    
    #[test]
    fn test_trade() {
        // 创建Agent和Product
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 100.0;
        let mut agent = Agent::new(agent_id, agent_name, initial_cash);
        
        let product_id = 1;
        
        // 设置Agent的偏好和需求
        let original_price = 50.0;
        let original_elastic = 1.0;
        let initial_range = (40.0, 60.0);
        
        {
            let mut preferences = agent.preferences.write().unwrap();
            let mut preference = Preference::new(original_price, original_elastic);
            preference.current_range = initial_range;
            preferences.insert(product_id, preference);
        }
        
        {  
            let mut demand = agent.demand.write().unwrap();
            demand.insert(product_id, true);
        }
        
        // 创建一个Product，确保其价格分布生成的supply_price_range与Agent的current_range有交集
        // 使用固定的正态分布，均值为50，标准差为5，确保生成的supply_price_range在40-60之间
        let product = crate::model::product::Product::from(
            product_id, 
            "test_product".to_string(),
            crate::entity::normal_distribute::NormalDistribution::new(50.0, product_id, "price_dist".to_string(), 5.0),
            crate::entity::normal_distribute::NormalDistribution::new(1.0, product_id, "elastic_dist".to_string(), 0.1)
        );
        
        // 创建Factory
        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory = crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);
        
        // 确保Factory的supply_price_range与Agent的current_range有交集
        let factory_range = factory.supply_price_range();
        
        // 获取Agent的range，使用独立的代码块确保借用及时释放
        let agent_range = {
            let preferences = agent.preferences.read().unwrap();
            preferences.get(&product_id).unwrap().current_range
        };
        
        // 计算交集，确保有交集
        let (agent_min, agent_max) = agent_range;
        let (factory_min, factory_max) = factory_range;
        let overlap_min = agent_min.max(factory_min);
        let overlap_max = agent_max.min(factory_max);
        
        // 如果没有交集，调整Agent的range，确保有交集
        if overlap_max <= overlap_min {
            let mut preferences = agent.preferences.write().unwrap();
            let preference = preferences.get_mut(&product_id).unwrap();
            // 设置一个与factory_range有交集的range
            preference.current_range = (factory_min, factory_max + 10.0);
        }
        
        // 记录交易前的状态
        let initial_cash = agent.cash();
        
        // 获取初始范围
        let initial_range = {
            let preferences_before = agent.preferences.read().unwrap();
            let preference_before = preferences_before.get(&product_id).unwrap();
            preference_before.current_range
        };
        
        // 执行交易
        let result = agent.trade(&factory);
        
        // 验证交易成功
        assert!(result.is_some(), "Trade should succeed with overlapping ranges");
        
        // 验证需求被移除
        {  
            let demand = agent.demand.read().unwrap();
            assert!(!demand.contains_key(&product_id), "Demand should be removed after successful trade");
        }
        
        // 验证current_price和current_range更新
        let preferences = agent.preferences.read().unwrap();
        let preference = preferences.get(&product_id).unwrap();
        
        // 验证cash减少
        assert!(agent.cash() < initial_cash, "Cash should decrease after trade");
        
        // 验证current_price被更新
        assert!(preference.current_price > 0.0, "Current price should be updated");
        
        // 验证current_range以新价格为中点，范围缩小10%
        let (new_min, new_max) = preference.current_range;
        let expected_length = (initial_range.1 - initial_range.0) * 0.9;
        let actual_length = new_max - new_min;
        assert!((actual_length - expected_length).abs() < expected_length * 0.1, "Range should be reduced by 10%"); // 允许10%的误差
        
        // 验证新范围以交易价格为中点
        let midpoint = (new_min + new_max) / 2.0;
        assert!((midpoint - preference.current_price).abs() < 0.001, "Range should be centered at trade price");
        
        // 验证范围不小于0
        assert!(new_min >= 0.0, "Range minimum should be >= 0");
        assert!(new_max > new_min, "Range maximum should be > minimum");
    }
    
    #[test]
    fn test_trade_insufficient_cash_but_in_range() {
        // 测试场景：现金不足但在交集范围内，应该用全部现金交易
        // 由于trade方法内部使用随机数生成价格，我们无法直接控制生成的价格
        // 因此我们需要多次尝试，直到得到我们想要的测试场景
        
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 50.0; // 现金不足
        
        let product_id = 1;
        
        // 多次尝试，直到交易成功或达到最大尝试次数
        let max_attempts = 100;
        let mut success = false;
        
        for _ in 0..max_attempts {
            // 每次尝试创建新的Agent和Factory
            let mut agent = Agent::new(agent_id, agent_name.clone(), initial_cash);
            
            // 设置Agent的偏好和需求，range为(40.0, 100.0)，确保cash(50.0)在范围内
            let original_price = 70.0;
            let original_elastic = 1.0;
            let initial_range = (40.0, 100.0);
            
            {
                let mut preferences = agent.preferences.write().unwrap();
                let mut preference = Preference::new(original_price, original_elastic);
                preference.current_range = initial_range;
                preferences.insert(product_id, preference);
            }
            
            {  
                let mut demand = agent.demand.write().unwrap();
                demand.insert(product_id, true);
            }
            
            // 创建Product，确保其price_distribution的均值为60.0，标准差为20.0
            let product = crate::model::product::Product::from(
                product_id, 
                "test_product".to_string(),
                crate::entity::normal_distribute::NormalDistribution::new(60.0, product_id, "price_dist".to_string(), 20.0),
                crate::entity::normal_distribute::NormalDistribution::new(1.0, product_id, "elastic_dist".to_string(), 0.1)
            );
            
            let factory_id = 1;
            let factory_name = "test_factory".to_string();
            let factory = crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);
            
            // 执行交易
            let result = agent.trade(&factory);
            
            // 检查是否交易成功且cash变为0
            if result.is_some() && agent.cash() == 0.0 {
                // 验证需求被移除
                let demand = agent.demand.read().unwrap();
                if !demand.contains_key(&product_id) {
                    success = true;
                    break;
                }
            }
        }
        
        // 确保至少有一次成功
        assert!(success, "Trade should succeed when cash is in range after multiple attempts");
    }
    
    #[test]
    fn test_trade_insufficient_cash_out_of_range() {
        // 测试场景：现金不足且不在交集范围内，应该走else分支
        
        let agent_id = 1;
        let agent_name = "test_agent".to_string();
        let initial_cash = 20.0; // 现金不足，且不在range内
        let mut agent = Agent::new(agent_id, agent_name, initial_cash);
        
        let product_id = 1;
        
        // 设置Agent的偏好和需求，range为(40.0, 60.0)，cash(20.0)不在范围内
        let original_price = 50.0;
        let original_elastic = 0.1; // 弹性小，删除概率低
        let initial_range = (40.0, 60.0);
        
        {
            let mut preferences = agent.preferences.write().unwrap();
            let mut preference = Preference::new(original_price, original_elastic);
            preference.current_range = initial_range;
            preferences.insert(product_id, preference);
        }
        
        {  
            let mut demand = agent.demand.write().unwrap();
            demand.insert(product_id, true);
        }
        
        // 创建Product，确保其supply_price_range与Agent的range有交集，但cash不在范围内
        let product = crate::model::product::Product::from(
            product_id, 
            "test_product".to_string(),
            crate::entity::normal_distribute::NormalDistribution::new(50.0, product_id, "price_dist".to_string(), 5.0),
            crate::entity::normal_distribute::NormalDistribution::new(1.0, product_id, "elastic_dist".to_string(), 0.1)
        );
        
        let factory_id = 1;
        let factory_name = "test_factory".to_string();
        let factory = crate::model::factory::Factory::new(factory_id, factory_name.clone(), &product);
        
        // 记录交易前的状态
        let initial_cash = agent.cash();
        let initial_range = {
            let preferences_before = agent.preferences.read().unwrap();
            preferences_before.get(&product_id).unwrap().current_range
        };
        
        // 执行交易
        let result = agent.trade(&factory);
        
        // 验证交易失败
        assert!(result.is_none(), "Trade should fail when cash is out of range");
        
        // 验证cash没有变化
        assert_eq!(agent.cash(), initial_cash, "Cash should not change after failed trade");
        
        // 验证range可能被更新（取决于随机概率）
        let preferences = agent.preferences.read().unwrap();
        let preference = preferences.get(&product_id).unwrap();
        let new_range = preference.current_range;
        
        // 由于有随机因素，我们只能验证range可能变化，不能断言一定会变化
        // 但可以验证range的有效性
        assert!(new_range.0 >= 0.0);
        assert!(new_range.1 > new_range.0);
    }
}
