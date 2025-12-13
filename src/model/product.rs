use std::fmt::{Debug, Formatter};
use crate::entity::normal_distribute::NormalDistribution;

#[derive(Clone)]
pub struct Product {
    id: u64,
    name: String,
    pub(crate) original_price_distribution: NormalDistribution,
    original_elastic_distribution: NormalDistribution,
    product_cost_distribution: NormalDistribution,
}

impl Debug for Product {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Product")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("original_price_distribution", &self.original_price_distribution)
            .field("original_elastic_distribution", &self.original_elastic_distribution)
            .field("product_cost_distribution", &self.product_cost_distribution)
            .finish()
    }
}

impl Product {
    pub fn new(id: u64, name: String) -> Self {
        let original_price_distribution =
            NormalDistribution::random(id, format!("{}_price_dist", name), Some(0.0), Some(1.0));

        let original_elastic_distribution =
            NormalDistribution::random(id, format!("{}_elastic_dist", name), Some(0.0), Some(1.0));

        let product_cost_distribution =
            NormalDistribution::random(id, format!("{}_cost_dist", name), Some(0.0), Some(1.0));

        Product {
            id,
            name,
            original_price_distribution,
            original_elastic_distribution,
            product_cost_distribution,
        }
    }

    pub fn from(
        id: u64,
        name: String,
        original_price_distribution: NormalDistribution,
        original_elastic_distribution: NormalDistribution,
        product_cost_distribution: NormalDistribution,
    ) -> Self {
        Product {
            id,
            name,
            original_price_distribution,
            original_elastic_distribution,
            product_cost_distribution,
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

    pub fn product_cost_distribution(&self) -> &NormalDistribution {
        &self.product_cost_distribution
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

        // 验证成本分布
        let cost_dist = product.product_cost_distribution();
        assert_eq!(cost_dist.id(), id);
        assert!(cost_dist.name().contains(&name));
        assert!(cost_dist.mean() >= 0.0);
    }

    #[test]
    fn test_from() {
        let id = 2;
        let name = "test_product_from".to_string();

        // 手动创建三个分布
        let price_dist = NormalDistribution::new(100.0, id, "price_dist".to_string(), 10.0);
        let elastic_dist = NormalDistribution::new(2.0, id, "elastic_dist".to_string(), 0.5);
        let cost_dist = NormalDistribution::new(50.0, id, "cost_dist".to_string(), 5.0);

        let product = Product::from(
            id,
            name.clone(),
            price_dist.clone(),
            elastic_dist.clone(),
            cost_dist.clone(),
        );

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

        // 验证成本分布与传入的一致
        let product_cost_dist = product.product_cost_distribution();
        assert_eq!(product_cost_dist.mean(), cost_dist.mean());
        assert_eq!(product_cost_dist.id(), cost_dist.id());
        assert_eq!(product_cost_dist.name(), cost_dist.name());
        assert_eq!(product_cost_dist.std_dev(), cost_dist.std_dev());
    }
}
