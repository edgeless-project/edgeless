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
    nodes: Vec<Uuid>,
}

impl OrchestrationLogic {
    pub fn new(orchestration_strategy: OrchestrationStrategy) -> Self {
        match orchestration_strategy {
            OrchestrationStrategy::Random => log::info!("Orchestration logic strategy: random"),
            OrchestrationStrategy::RoundRobin => log::info!("Orchestration logic strategy: round-robin"),
        };

        Self {
            orchestration_strategy,
            round_robin_current_index: 0,
            rng: StdRng::from_entropy(),
            nodes: vec![],
        }
    }

    pub fn update_nodes(&mut self, nodes: Vec<Uuid>) -> () {
        self.nodes = nodes;
        self.round_robin_current_index = 0;
    }
}

/// This iterator can be used to select the next node on which a function
/// instance should be spawned, based on a general orchestration strategy as
/// defined in the settings.
impl Iterator for OrchestrationLogic {
    type Item = Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        match self.orchestration_strategy {
            OrchestrationStrategy::Random => self.nodes.choose(&mut self.rng).cloned(),
            OrchestrationStrategy::RoundRobin => {
                if self.round_robin_current_index >= self.nodes.len() {
                    self.round_robin_current_index = 0;
                }
                let next_node = Some(self.nodes[self.round_robin_current_index]);
                self.round_robin_current_index += 1;
                next_node
            }
        }
    }
}
