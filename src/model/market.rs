use crate::logging::LOGGER;
use crate::model::agent::{Agent, IntervalRelation, TradeResult};
use crate::model::consumer_association::ConsumerAssociation;
use crate::model::factory::{Factory, FactoryStatus};
use crate::model::product::Product;
use crate::model::producer_association::ProducerAssociation;
use parking_lot::RwLock;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_TICKS: u64 = 48_000_000;
const MAX_RETRIES: usize = 100;

pub struct Market {
    consumers: ConsumerAssociation,
    producers: ProducerAssociation,
    products: Vec<Product>,
    tick: u64,
}

impl Market {
    pub fn new(products: Vec<Product>) -> Self {
        let mut rng = rand::thread_rng();
        let mut factories_map: HashMap<u64, Arc<RwLock<Factory>>> = HashMap::new();
        let mut agents_vec: Vec<Arc<RwLock<Agent>>> = Vec::new();
        let mut factory_id_counter: u64 = 1;

        println!("Creating factories...");
        for product in &products {
            let factory_count = if rng.gen_bool(0.5) { 3 } else { 4 };
            for i in 0..factory_count {
                let factory = Factory::new(
                    factory_id_counter,
                    format!("{}_{}", product.name(), i),
                    product,
                );
                let factory_arc = Arc::new(RwLock::new(factory));
                Factory::start_production_thread(factory_arc.clone());
                factories_map.insert(factory_id_counter, factory_arc);
                factory_id_counter += 1;
            }
        }

        println!("Creating {} agents...", 300);
        for agent_id in 1..=300 {
            let agent = Agent::new(
                agent_id,
                format!("Consumer_{}", agent_id),
                10000.0,
                &products,
                true,
            );
            let agent_arc = Arc::new(RwLock::new(agent));
            Agent::start_ubi_thread(agent_arc.clone());
            agents_vec.push(agent_arc);
        }
        println!("All agents created.");

        Market {
            consumers: ConsumerAssociation::new(agents_vec),
            producers: ProducerAssociation::new(factories_map),
            products,
            tick: 0,
        }
    }

    pub fn run(&mut self) {
        let mut rng = rand::thread_rng();
        let mut total_trades: u64 = 0;
        let snapshot_interval: u64 = 1000;

        for tick in 0..MAX_TICKS {
            self.tick = tick;

            let factory_arc = match self.producers.random_active_factory(&mut rng, MAX_RETRIES) {
                Some(f) => f,
                None => {
                    if self.should_terminate() {
                        break;
                    }
                    continue;
                }
            };

            let product_id = factory_arc.read().product_id();

            let agent_arc = match self
                .consumers
                .random_agent_with_demand(product_id, &mut rng, MAX_RETRIES)
            {
                Some(a) => a,
                None => {
                    if self.should_terminate() {
                        break;
                    }
                    continue;
                }
            };

            if self.try_trade(tick, agent_arc, factory_arc) {
                total_trades += 1;
            }

            if tick % snapshot_interval == 0 {
                self.log_snapshot(tick, total_trades);
            }

            if tick % 100_000 == 0 && tick > 0 {
                println!(
                    "Tick: {}, Total trades: {}, Agents: {}, Factories: {}",
                    tick,
                    total_trades,
                    self.consumers.agent_count(),
                    self.producers.factory_count(),
                );
            }
        }

        println!(
            "Simulation completed at tick {}, total trades: {}",
            self.tick, total_trades
        );
    }

    fn try_trade(
        &self,
        tick: u64,
        agent_arc: Arc<RwLock<Agent>>,
        factory_arc: Arc<RwLock<Factory>>,
    ) -> bool {
        let product_id;
        let product_category;
        let price;
        let factory_id;
        let factory_name;

        {
            let factory = factory_arc.read();
            if factory.status() == FactoryStatus::BrokeUp || factory.stock == 0 {
                return false;
            }
            product_id = factory.product_id();
            product_category = factory.product_category();
            price = factory.offer_price();
            factory_id = factory.id();
            factory_name = factory.name().to_string();
            if price == 0.0 {
                return false;
            }
        }

        let (result, interval_relation) = {
            let agent = agent_arc.read();
            agent.negotiate(tick, product_id, product_category, price)
        };

        {
            let mut agent = agent_arc.write();
            agent.settling(product_id, product_category, tick, result, vec![price]);
        }

        let stock_low;
        {
            let mut factory = factory_arc.write();
            factory.resolve_trade(&result, tick, Some(interval_relation));
            stock_low = factory.stock_ratio() < 0.25;

            if factory.should_produce_on_cycle(tick) {
                factory.try_produce(tick);
                factory.apply_decay();
            }
        }

        // Log the trade
        if matches!(result, TradeResult::Success(_)) || matches!(result, TradeResult::Failed) {
            let factory = factory_arc.read();
            let agent = agent_arc.read();
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as i64;
            let mut logger = LOGGER.write();
            if let Err(e) = logger.log_trade(
                timestamp,
                tick,
                0,
                agent.id(),
                agent.name().to_string(),
                agent.cash(),
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                &factory,
                &Product::new(product_id, "".to_string(), product_category.clone(), 1.0),
                &result,
                format!("{:?}", interval_relation).as_str(),
            ) {
                eprintln!("Failed to log trade: {}", e);
            }
            drop(logger);
        }

        // Trigger immediate production if stock low after successful trade
        if matches!(result, TradeResult::Success(_)) && stock_low {
            let mut factory = factory_arc.write();
            factory.try_produce(tick);
        }

        matches!(result, TradeResult::Success(_))
    }

    fn should_terminate(&self) -> bool {
        if self.consumers.all_broke_up() {
            println!(
                "Terminating: all agents have zero cash at tick {}",
                self.tick
            );
            return true;
        }
        false
    }

    fn log_snapshot(&self, tick: u64, total_trades: u64) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_millis() as i64;
        let mut logger = LOGGER.write();
        // Log agent cash snapshot for each agent
        // This uses the existing log_agent_cash function
        // Skipped for now to reduce log volume; can be enabled if needed
        drop(logger);
    }
}
