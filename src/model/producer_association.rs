use crate::model::factory::{Factory, FactoryStatus};
use parking_lot::RwLock;
use rand::Rng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProducerAssociation {
    factories: HashMap<u64, Arc<RwLock<Factory>>>,
    factory_ids: Vec<u64>,
    product_to_factories: HashMap<u64, Vec<u64>>,
}

impl ProducerAssociation {
    pub fn new(factories: HashMap<u64, Arc<RwLock<Factory>>>) -> Self {
        let mut factory_ids: Vec<u64> = factories.keys().copied().collect();
        factory_ids.sort();

        let mut product_to_factories: HashMap<u64, Vec<u64>> = HashMap::new();
        for (&id, factory_arc) in &factories {
            let product_id = factory_arc.read().product_id();
            product_to_factories
                .entry(product_id)
                .or_default()
                .push(id);
        }

        Self {
            factories,
            factory_ids,
            product_to_factories,
        }
    }

    pub fn random_factory(&self, rng: &mut impl Rng) -> Option<Arc<RwLock<Factory>>> {
        if self.factory_ids.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.factory_ids.len());
        let id = self.factory_ids[idx];
        self.factories.get(&id).cloned()
    }

    pub fn random_active_factory(
        &self,
        rng: &mut impl Rng,
        max_retries: usize,
    ) -> Option<Arc<RwLock<Factory>>> {
        let len = self.factory_ids.len();
        if len == 0 {
            return None;
        }
        for _ in 0..max_retries {
            let idx = rng.gen_range(0..len);
            let id = self.factory_ids[idx];
            if let Some(factory) = self.factories.get(&id) {
                let f = factory.read();
                if f.status() == FactoryStatus::Active && f.stock > 0 {
                    return Some(factory.clone());
                }
            }
        }
        self.factory_ids.iter().find_map(|id| {
            let f = self.factories.get(id)?.read();
            if f.status() == FactoryStatus::Active && f.stock > 0 {
                Some(self.factories.get(id).unwrap().clone())
            } else {
                None
            }
        })
    }

    pub fn random_active_factories_for_product(
        &self,
        product_id: u64,
        n: usize,
        rng: &mut impl Rng,
    ) -> Vec<Arc<RwLock<Factory>>> {
        let candidate_ids = match self.product_to_factories.get(&product_id) {
            Some(ids) => ids,
            None => return vec![],
        };

        let mut active: Vec<&u64> = candidate_ids
            .iter()
            .filter(|&&id| {
                if let Some(factory) = self.factories.get(&id) {
                    let f = factory.read();
                    f.status() == FactoryStatus::Active && f.stock > 0
                } else {
                    false
                }
            })
            .collect();

        if active.is_empty() {
            return vec![];
        }

        active.shuffle(rng);
        let count = active.len().min(n);

        let mut results: Vec<(f64, Arc<RwLock<Factory>>)> = active[..count]
            .iter()
            .map(|&&id| {
                let f = self.factories.get(&id).unwrap().read();
                let price = f.offer_price();
                (price, self.factories.get(&id).unwrap().clone())
            })
            .collect();

        results.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        results.into_iter().map(|(_, arc)| arc).collect()
    }

    pub fn factory_count(&self) -> usize {
        self.factory_ids.len()
    }

    pub fn factory_count_for_product(&self, product_id: u64) -> usize {
        self.product_to_factories
            .get(&product_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::product::{Product, ProductCategory};

    fn make_test_factory(id: u64, product_id: u64) -> Arc<RwLock<Factory>> {
        let product = Product::new(product_id, format!("p{}", product_id), ProductCategory::Food, 1.0);
        let factory = Factory::new(id, format!("F_{}", id), &product);
        Arc::new(RwLock::new(factory))
    }

    #[test]
    fn test_empty_association() {
        let mut rng = rand::thread_rng();
        let assoc = ProducerAssociation::new(HashMap::new());
        assert_eq!(assoc.factory_count(), 0);
        assert!(assoc.random_factory(&mut rng).is_none());
        assert!(assoc.random_active_factory(&mut rng, 10).is_none());
        assert_eq!(assoc.factory_count_for_product(1), 0);
    }

    #[test]
    fn test_random_factory() {
        let mut rng = rand::thread_rng();
        let mut factories = HashMap::new();
        for i in 1..=10 {
            factories.insert(i as u64, make_test_factory(i as u64, 1));
        }
        let assoc = ProducerAssociation::new(factories);

        assert_eq!(assoc.factory_count(), 10);
        for _ in 0..30 {
            assert!(assoc.random_factory(&mut rng).is_some());
        }
    }

    #[test]
    fn test_random_active_factory() {
        let mut rng = rand::thread_rng();
        let mut factories = HashMap::new();
        for i in 1..=5 {
            let f = make_test_factory(i as u64, 1);
            {
                let mut w = f.write();
                w.stock = 10;
            }
            factories.insert(i as u64, f);
        }
        // Add one broke factory
        let broken = make_test_factory(6, 1);
        {
            let mut w = broken.write();
            w.stock = 0;
        }
        factories.insert(6, broken);

        let assoc = ProducerAssociation::new(factories);

        for _ in 0..20 {
            let result = assoc.random_active_factory(&mut rng, 50);
            assert!(result.is_some());
            let f = result.unwrap();
            assert_eq!(f.read().status(), FactoryStatus::Active);
            assert!(f.read().stock > 0);
        }
    }

    #[test]
    fn test_random_active_for_product() {
        let mut rng = rand::thread_rng();
        let mut factories = HashMap::new();
        for i in 1..=4 {
            let f = make_test_factory(i as u64, 1);
            {
                let mut w = f.write();
                w.stock = 10;
            }
            factories.insert(i as u64, f);
        }
        for i in 5..=6 {
            let f = make_test_factory(i as u64, 2);
            {
                let mut w = f.write();
                w.stock = 10;
            }
            factories.insert(i as u64, f);
        }

        let assoc = ProducerAssociation::new(factories);
        assert_eq!(assoc.factory_count_for_product(1), 4);
        assert_eq!(assoc.factory_count_for_product(2), 2);

        let result = assoc.random_active_factories_for_product(1, 3, &mut rng);
        assert!(result.len() <= 3);
        for f in &result {
            assert_eq!(f.read().product_id(), 1);
        }
    }
}
