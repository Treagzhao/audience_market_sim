use crate::entity::normal_distribute::NormalDistribution;

#[derive(Clone)]
pub struct Product {
    id: u64,
    name: String,
    original_price_distribution: NormalDistribution,
    original_elastic_distribution: NormalDistribution,
}

impl Product {
    pub fn new(id: u64, name: String) -> Self {
        let original_price_distribution =
            NormalDistribution::random(id, format!("{}_price_dist", name), None, None);

        let original_elastic_distribution =
            NormalDistribution::random(id, format!("{}_elastic_dist", name), None, None);

        Product {
            id,
            name,
            original_price_distribution,
            original_elastic_distribution,
        }
    }

    pub fn from(
        id: u64,
        name: String,
        original_price_distribution: NormalDistribution,
        original_elastic_distribution: NormalDistribution,
    ) -> Self {
        Product {
            id,
            name,
            original_price_distribution,
            original_elastic_distribution,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn original_price_distribution(&self) -> &NormalDistribution {
        &self.original_price_distribution
    }

    pub fn original_elastic_distribution(&self) -> &NormalDistribution {
        &self.original_elastic_distribution
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let id = 1;
        let name = "test_product".to_string();

        let product = Product::new(id, name.clone());

        assert_eq!(product.id(), id);
        assert_eq!(product.name(), name);

        // 验证价格分布
        let price_dist = product.original_price_distribution();
        assert_eq!(price_dist.id(), id);
        assert!(price_dist.name().contains(&name));
        assert!(price_dist.mean() >= 0.0);

        // 验证弹性分布
        let elastic_dist = product.original_elastic_distribution();
        assert_eq!(elastic_dist.id(), id);
        assert!(elastic_dist.name().contains(&name));
        assert!(elastic_dist.mean() >= 0.0);
    }

    #[test]
    fn test_from() {
        let id = 2;
        let name = "test_product_from".to_string();

        // 手动创建两个分布
        let price_dist = NormalDistribution::new(100.0, id, "price_dist".to_string(), 10.0);
        let elastic_dist = NormalDistribution::new(2.0, id, "elastic_dist".to_string(), 0.5);

        let product = Product::from(id, name.clone(), price_dist.clone(), elastic_dist.clone());

        assert_eq!(product.id(), id);
        assert_eq!(product.name(), name);

        // 验证价格分布与传入的一致
        let product_price_dist = product.original_price_distribution();
        assert_eq!(product_price_dist.mean(), price_dist.mean());
        assert_eq!(product_price_dist.id(), price_dist.id());
        assert_eq!(product_price_dist.name(), price_dist.name());
        assert_eq!(product_price_dist.std_dev(), price_dist.std_dev());

        // 验证弹性分布与传入的一致
        let product_elastic_dist = product.original_elastic_distribution();
        assert_eq!(product_elastic_dist.mean(), elastic_dist.mean());
        assert_eq!(product_elastic_dist.id(), elastic_dist.id());
        assert_eq!(product_elastic_dist.name(), elastic_dist.name());
        assert_eq!(product_elastic_dist.std_dev(), elastic_dist.std_dev());
    }
}
