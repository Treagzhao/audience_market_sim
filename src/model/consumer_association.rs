use crate::model::agent::Agent;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::Arc;

pub struct ConsumerAssociation {
    agents: Vec<Arc<RwLock<Agent>>>,
}

impl ConsumerAssociation {
    pub fn new(agents: Vec<Arc<RwLock<Agent>>>) -> Self {
        Self { agents }
    }

    pub fn random_agent(&self, rng: &mut impl Rng) -> Option<Arc<RwLock<Agent>>> {
        if self.agents.is_empty() {
            return None;
        }
        let idx = rng.gen_range(0..self.agents.len());
        Some(self.agents[idx].clone())
    }

    pub fn random_agent_with_demand(
        &self,
        product_id: u64,
        rng: &mut impl Rng,
        max_retries: usize,
    ) -> Option<Arc<RwLock<Agent>>> {
        let len = self.agents.len();
        if len == 0 {
            return None;
        }
        for _ in 0..max_retries {
            let idx = rng.gen_range(0..len);
            if self.agents[idx].read().has_demand(product_id) {
                return Some(self.agents[idx].clone());
            }
        }
        self.agents
            .iter()
            .find(|a| a.read().has_demand(product_id))
            .cloned()
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn all_broke_up(&self) -> bool {
        self.agents.iter().all(|a| a.read().cash() < 0.01)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::normal_distribute::NormalDistribution;
    use crate::model::product::{Product, ProductCategory};

    fn make_test_agent(id: u64, cash: f64, products: &[Product]) -> Arc<RwLock<Agent>> {
        Arc::new(RwLock::new(Agent::new(
            id,
            format!("Agent_{}", id),
            cash,
            products,
            false,
        )))
    }

    #[test]
    fn test_empty_association() {
        let assoc = ConsumerAssociation::new(vec![]);
        let mut rng = rand::thread_rng();
        assert_eq!(assoc.agent_count(), 0);
        assert!(assoc.random_agent(&mut rng).is_none());
        assert!(assoc
            .random_agent_with_demand(1, &mut rng, 10)
            .is_none());
        assert!(assoc.all_broke_up());
    }

    #[test]
    fn test_random_agent() {
        let products = vec![Product::new(1, "p1".to_string(), ProductCategory::Food, 1.0)];
        let agents: Vec<_> = (1..=10)
            .map(|i| make_test_agent(i, 100.0, &products))
            .collect();
        let assoc = ConsumerAssociation::new(agents);
        let mut rng = rand::thread_rng();

        assert_eq!(assoc.agent_count(), 10);
        for _ in 0..20 {
            assert!(assoc.random_agent(&mut rng).is_some());
        }
    }

    #[test]
    fn test_random_agent_with_demand() {
        let products = vec![Product::new(1, "p1".to_string(), ProductCategory::Food, 1.0)];
        let agents: Vec<_> = (1..=10)
            .map(|i| make_test_agent(i, 100.0, &products))
            .collect();

        // Only agent 5 has demand for product 1
        {
            let mut a = agents[4].write();
            a.set_demand(1, true);
        }

        let assoc = ConsumerAssociation::new(agents);
        let mut rng = rand::thread_rng();

        for _ in 0..20 {
            let result = assoc.random_agent_with_demand(1, &mut rng, 100);
            assert!(result.is_some());
            assert_eq!(result.unwrap().read().id(), 5);
        }

        // No one has demand for product 2
        assert!(assoc
            .random_agent_with_demand(2, &mut rng, 100)
            .is_none());
    }

    #[test]
    fn test_all_broke_up() {
        let products = vec![Product::new(1, "p1".to_string(), ProductCategory::Food, 1.0)];
        let agents: Vec<_> = (1..=3)
            .map(|i| make_test_agent(i, 0.0, &products))
            .collect();

        let assoc = ConsumerAssociation::new(agents);
        assert!(assoc.all_broke_up());

        let agents2: Vec<_> = (1..=3)
            .map(|i| make_test_agent(i, 100.0, &products))
            .collect();
        let assoc2 = ConsumerAssociation::new(agents2);
        assert!(!assoc2.all_broke_up());
    }
}
