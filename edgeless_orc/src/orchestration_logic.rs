use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};
use uuid::Uuid;

use crate::OrchestrationStrategy;

/// Keeps all the necessary state that is needed to make simple orchestration
/// decisions. Provides convenience methods that can be used by the
/// orchestrator.
pub struct OrchestrationLogic {
    orchestration_strategy: OrchestrationStrategy,
    round_robin_current_index: usize,
    rng: StdRng,
    orchestrated_nodes: Vec<Uuid>,
}

impl OrchestrationLogic {
    pub fn new(orchestration_strategy: Option<OrchestrationStrategy>, orchestrated_nodes: Vec<Uuid>) -> Self {
        Self {
            orchestration_strategy: match orchestration_strategy {
                Some(s) => s,
                None => OrchestrationStrategy::Random,
            },
            round_robin_current_index: 0,
            rng: StdRng::from_entropy(),
            orchestrated_nodes,
        }
    }
}

/// This iterator can be used to select the next node on which a function
/// instance should be spawned, based on a general orchestration strategy as
/// defined in the settings.
impl Iterator for OrchestrationLogic {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        match self.orchestration_strategy {
            OrchestrationStrategy::Random => {
                log::info!("Orchestration Logic used Random strategy");
                self.orchestrated_nodes.choose(&mut self.rng).cloned()
            }
            OrchestrationStrategy::RoundRobin => {
                log::info!("Orchestration Logic used RoundRobin strategy");
                if self.round_robin_current_index >= self.orchestrated_nodes.len() {
                    self.round_robin_current_index = 0;
                }
                let next_node = Some(self.orchestrated_nodes[self.round_robin_current_index]);
                self.round_robin_current_index += 1;
                next_node
            }
        }
    }
}
