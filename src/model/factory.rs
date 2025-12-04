use std::collections::{HashMap, LinkedList};
use rand::Rng;
use crate::model::product::Product;

pub struct Factory{
    id:u64,
    name:String,
    product_id:u64,
    supply_price_range:(f64,f64),
    amount:HashMap<u64,i16>,
    u64_list:LinkedList<u64>
}

impl Factory {
    pub fn new(id: u64, name: String, product: &Product) -> Self {
        // 根据商品的价格正态分布，随机生成一个参考价格点
        let reference_price = product.original_price_distribution().sample(None);

        // 随机生成区间，上下界都是随机的，下界不能小于0.0
        let mut rng = rand::thread_rng();

        // 处理特殊情况，确保reference_price是有限值
        let reference_price = reference_price.min(f64::MAX / 2.0);

        // 计算区间范围，基于参考价格
        let range_scale = reference_price * 0.5;

        // 生成下界：0.0到reference_price
        let lower = rng.gen_range(0.0..reference_price);

        // 确保upper_bound是有限值
        let upper_bound = (reference_price + range_scale).min(f64::MAX / 2.0);
        // 生成上界：lower到upper_bound
        let upper = rng.gen_range(lower..upper_bound);

        Self {
            id,
            name,
            product_id: product.id(),
            supply_price_range: (lower, upper),
            amount: HashMap::new(),
            u64_list: LinkedList::new(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn product_id(&self) -> u64 {
        self.product_id
    }

    pub fn supply_price_range(&self) -> (f64, f64) {
        self.supply_price_range
    }

    /// 开始新一轮
    pub fn start_round(&mut self, round: u64) {
        // 给hashmap创建一个以round为键，值为10的数字
        self.amount.insert(round, 10);
        
        // 把round插入到队尾
        self.u64_list.push_back(round);
        
        // 队列长度超过3就从队首弹出
        if self.u64_list.len() > 3 {
            let v = self.u64_list.pop_front();
            if let Some(v) = v {
                self.amount.remove(&v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::product::Product;
    
    #[test]
    fn test_new() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        
        // 验证初始化后的字段值
        assert_eq!(factory.id(), 1);
        assert_eq!(factory.name(), "test_factory");
        assert_eq!(factory.product_id(), 1);
        assert_eq!(factory.amount.len(), 0);
        assert_eq!(factory.u64_list.len(), 0);
        
        // 验证supply_price_range是有效的
        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }
    
    #[test]
    fn test_id() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(42, "test_factory".to_string(), &product);
        assert_eq!(factory.id(), 42);
    }
    
    #[test]
    fn test_name() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "my_factory".to_string(), &product);
        assert_eq!(factory.name(), "my_factory");
    }
    
    #[test]
    fn test_product_id() {
        let product = Product::new(5, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        assert_eq!(factory.product_id(), 5);
    }
    
    #[test]
    fn test_supply_price_range() {
        let product = Product::new(1, "test_product".to_string());
        let factory = Factory::new(1, "test_factory".to_string(), &product);
        let (lower, upper) = factory.supply_price_range();
        assert!(lower >= 0.0);
        assert!(upper > lower);
    }
    
    #[test]
    fn test_start_round() {
        // 创建一个Product实例用于初始化Factory
        let product = Product::new(1, "test_product".to_string());
        let mut factory = Factory::new(1, "test_factory".to_string(), &product);
        
        // 测试第一轮
        factory.start_round(1);
        assert_eq!(factory.amount.get(&1), Some(&10));
        assert_eq!(factory.u64_list.len(), 1);
        
        // 测试第二轮
        factory.start_round(2);
        assert_eq!(factory.amount.get(&2), Some(&10));
        assert_eq!(factory.u64_list.len(), 2);
        
        // 测试第三轮
        factory.start_round(3);
        assert_eq!(factory.amount.get(&3), Some(&10));
        assert_eq!(factory.u64_list.len(), 3);
        
        // 测试第四轮，此时队列长度超过3，应该弹出第一个元素
        factory.start_round(4);
        assert_eq!(factory.amount.get(&4), Some(&10));
        assert_eq!(factory.u64_list.len(), 3);
        // 第一个元素(1)应该被弹出并从amount中移除
        assert!(factory.amount.get(&1).is_none());
        
        // 测试第五轮，此时队列长度超过3，应该弹出第二个元素
        factory.start_round(5);
        assert_eq!(factory.amount.get(&5), Some(&10));
        assert_eq!(factory.u64_list.len(), 3);
        // 第二个元素(2)应该被弹出并从amount中移除
        assert!(factory.amount.get(&2).is_none());
        
        // 验证当前amount中只有3、4、5三个键
        assert_eq!(factory.amount.len(), 3);
        assert!(factory.amount.contains_key(&3));
        assert!(factory.amount.contains_key(&4));
        assert!(factory.amount.contains_key(&5));
        
        // 验证队列中的元素顺序
        let mut iter = factory.u64_list.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }
}