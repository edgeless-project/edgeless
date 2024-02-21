// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use rand::distributions::Distribution;
use rand::SeedableRng;

/// Keeps all the necessary state that is needed to make simple orchestration
/// decisions. Provides convenience methods that can be used by the
/// orchestrator.
pub struct OrchestrationLogic {
    /// Orchestration strategy.
    orchestration_strategy: crate::OrchestrationStrategy,
    /// Used by RoundRobin. Current index in vector nodes (circular).
    round_robin_current_index: usize,
    /// Random-number generator.
    rng: rand::rngs::StdRng,
    /// Vector of the nodes that can be selected.
    nodes: Vec<uuid::Uuid>,
    /// Used by Random, pair of (weight, node_id).
    weights: Vec<f32>,
    /// Used by Random.
    weight_dist: rand::distributions::Uniform<f32>,
}

impl OrchestrationLogic {
    pub fn new(orchestration_strategy: crate::OrchestrationStrategy) -> Self {
        match orchestration_strategy {
            crate::OrchestrationStrategy::Random => log::info!("Orchestration logic strategy: random"),
            crate::OrchestrationStrategy::RoundRobin => log::info!("Orchestration logic strategy: round-robin"),
        };

        Self {
            orchestration_strategy,
            round_robin_current_index: 0,
            rng: rand::rngs::StdRng::from_entropy(),
            nodes: vec![],
            weights: vec![],
            weight_dist: rand::distributions::Uniform::new(0.0, 1.0),
        }
    }

    pub fn update_nodes(&mut self, clients: &std::collections::HashMap<uuid::Uuid, crate::orchestrator::ClientDesc>) {
        // Refresh the nodes and weights data structures with the current set of nodes and their capabilities.
        self.nodes.clear();
        self.weights.clear();
        for (node, desc) in clients {
            if desc.capabilities.do_not_use() {
                // Skip the node if it must not be used, no matter what.
                continue;
            }
            self.nodes.push(*node);
            let mut weight = desc.capabilities.num_cores as f32 * desc.capabilities.num_cpus as f32 * desc.capabilities.clock_freq_cpu;
            if weight == 0.0 {
                // Force a vanishing weight to an arbitrary value.
                weight = 1.0;
            }
            self.weights.push(weight);
        }
        assert!(self.nodes.len() == self.weights.len());
        assert!(self.nodes.len() <= clients.len());

        // Initialize the orchestration variables depending on the strategy
        match self.orchestration_strategy {
            crate::OrchestrationStrategy::Random => {
                let high = match self.nodes.is_empty() {
                    true => 1.0,
                    false => self.weights.iter().sum::<f32>(),
                };
                self.weight_dist = rand::distributions::Uniform::new(0.0, high);
            }
            crate::OrchestrationStrategy::RoundRobin => self.round_robin_current_index = 0,
        };
    }
}

/// This iterator can be used to select the next node on which a function
/// instance should be spawned, based on a general orchestration strategy as
/// defined in the settings.
impl Iterator for OrchestrationLogic {
    type Item = uuid::Uuid;

    fn next(&mut self) -> Option<Self::Item> {
        if self.nodes.is_empty() {
            return None;
        }
        match self.orchestration_strategy {
            crate::OrchestrationStrategy::Random => {
                assert!(self.nodes.len() == self.weights.len());
                let rnd = self.weight_dist.sample(&mut self.rng);
                let mut sum = 0.0_f32;
                for i in 0..self.nodes.len() {
                    sum += self.weights[i];
                    if sum >= rnd {
                        return Some(self.nodes[i]);
                    }
                }
                self.nodes.last().cloned()
            }
            crate::OrchestrationStrategy::RoundRobin => {
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
