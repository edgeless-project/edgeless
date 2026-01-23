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
    /// Capabilities of the nodes.
    capabilities: Vec<edgeless_api::node_registration::NodeCapabilities>,
    /// Resource providers of the nodes.
    resource_providers: Vec<std::collections::HashSet<String>>,
    /// Used by Random, pair of (weight, node_id).
    weights: Vec<f32>,
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
            capabilities: vec![],
            resource_providers: vec![],
            weights: vec![],
        }
    }

    pub fn update_nodes(
        &mut self,
        clients: &std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>,
        resource_providers: &std::collections::HashMap<String, crate::resource_provider::ResourceProvider>,
    ) {
        // Refresh the nodes and weights data structures with the current set of nodes and their capabilities.
        self.nodes.clear();
        self.capabilities.clear();
        self.resource_providers.clear();
        self.weights.clear();
        for (node, desc) in clients {
            if desc.capabilities.do_not_use() || desc.cordoned {
                // Skip the node if it must not be used, no matter what.
                continue;
            }
            self.nodes.push(*node);
            self.capabilities.push(desc.capabilities.clone());
            self.resource_providers.push(
                resource_providers
                    .iter()
                    .filter(|(_, info)| info.node_id == *node)
                    .map(|(name, _)| name.clone())
                    .collect(),
            );
            let mut weight = (std::cmp::max(desc.capabilities.num_cores, desc.capabilities.num_cpus) as f32) * desc.capabilities.clock_freq_cpu;
            if weight == 0.0 {
                // Force a vanishing weight to an arbitrary value.
                weight = 1.0;
            }
            self.weights.push(weight);
        }
        assert!(self.nodes.len() == self.capabilities.len());
        assert!(self.nodes.len() == self.resource_providers.len());
        assert!(self.nodes.len() == self.weights.len());
        assert!(self.nodes.len() <= clients.len());
    }

    /// Filter only the nodes on which the given function can be deployed.
    pub fn feasible_nodes(&self, spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest, nodes: &Vec<uuid::Uuid>) -> Vec<uuid::Uuid> {
        let mut candidates = vec![];

        for candidate in nodes {
            if let Some(ndx) = self.nodes.iter().position(|&x| x == *candidate) {
                if OrchestrationLogic::is_node_feasible(
                    &spawn_req.spec.function_type,
                    &crate::deployment_requirements::DeploymentRequirements::from_annotations(&spawn_req.annotations),
                    &self.nodes[ndx],
                    &self.capabilities[ndx],
                    &self.resource_providers[ndx],
                ) {
                    candidates.push(self.nodes[ndx]);
                }
            }
        }

        candidates
    }

    /// Return true if it is possible to assign a function requesting a given
    /// run-time and with given deployment requirements to a node with
    /// given UUID and capabilities.
    pub fn is_node_feasible(
        runtime: &str,
        reqs: &crate::deployment_requirements::DeploymentRequirements,
        node_id: &uuid::Uuid,
        capabilities: &edgeless_api::node_registration::NodeCapabilities,
        resource_providers: &std::collections::HashSet<String>,
    ) -> bool {
        capabilities.runtimes.contains(&runtime.to_string()) && reqs.is_feasible(node_id, capabilities, resource_providers)
    }

    /// Select the next node on which a function instance should be spawned,
    /// based on a general orchestration strategy as defined in the settings.
    /// Always match the deployment requirements specified with the nodes'
    /// capabilities.
    pub fn next(&mut self, spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest) -> Option<uuid::Uuid> {
        if self.nodes.is_empty() {
            return None;
        }
        let reqs = crate::deployment_requirements::DeploymentRequirements::from_annotations(&spawn_req.annotations);
        match self.orchestration_strategy {
            crate::OrchestrationStrategy::Random => {
                // Select only the nodes that are feasible.
                let mut candidates = vec![];
                let mut high: f32 = 0.0;
                for i in 0..self.nodes.len() {
                    if Self::is_node_feasible(
                        &spawn_req.spec.function_type,
                        &reqs,
                        &self.nodes[i],
                        &self.capabilities[i],
                        &self.resource_providers[i],
                    ) {
                        candidates.push((i, self.weights[i]));
                        high += self.weights[i];
                    }
                }
                if high > 0.0 {
                    let rv = rand::distributions::Uniform::new(0.0, high);
                    let rnd = rv.sample(&mut self.rng);
                    let mut sum = 0.0_f32;
                    for candidate in candidates {
                        sum += candidate.1;
                        if sum >= rnd {
                            return Some(self.nodes[candidate.0]);
                        }
                    }
                }
                None
            }
            crate::OrchestrationStrategy::RoundRobin => {
                // Prevent infinite loop: evaluate each node at most once.
                for _ in 0..self.nodes.len() {
                    // Wrap-around if the current index is out of bounds.
                    if self.round_robin_current_index >= self.nodes.len() {
                        self.round_robin_current_index = 0;
                    }

                    let cand_ndx = self.round_robin_current_index;
                    self.round_robin_current_index += 1;

                    if Self::is_node_feasible(
                        &spawn_req.spec.function_type,
                        &reqs,
                        &self.nodes[cand_ndx],
                        &self.capabilities[cand_ndx],
                        &self.resource_providers[cand_ndx],
                    ) {
                        return Some(self.nodes[cand_ndx]);
                    }
                }
                None
            }
        }
    }

    pub fn next_excluding(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
        exclude: &Vec<uuid::Uuid>,
    ) -> Option<uuid::Uuid> {
        let feasible_nodes = self.feasible_nodes(spawn_req, &self.nodes);
        let candidates: Vec<uuid::Uuid> = feasible_nodes.into_iter().filter(|node_id| !exclude.contains(node_id)).collect();
        if candidates.is_empty() {
            return None;
        }
        match self.orchestration_strategy {
            crate::OrchestrationStrategy::Random => {
                let rv = rand::distributions::Uniform::new(0, candidates.len());
                let rnd = rv.sample(&mut self.rng);
                Some(candidates[rnd])
            }
            crate::OrchestrationStrategy::RoundRobin => {
                // Prevent infinite loop: evaluate each node at most once.
                if let Some(_) = (0..candidates.len()).next() {
                    // Wrap-around if the current index is out of bounds.
                    if self.round_robin_current_index >= candidates.len() {
                        self.round_robin_current_index = 0;
                    }

                    let cand_ndx = self.round_robin_current_index;
                    self.round_robin_current_index += 1;

                    return Some(candidates[cand_ndx]);
                }
                None
            }
        }
    }
}

/// Tests
#[cfg(test)]
mod tests {

    use crate::affinity_level::AffinityLevel;
    use crate::deployment_requirements::DeploymentRequirements;

    #[test]
    fn test_orchestration_logic_is_node_feasible() {
        let node_id = uuid::Uuid::new_v4();
        let mut reqs = DeploymentRequirements::none();
        let mut caps = edgeless_api::node_registration::NodeCapabilities::minimum();
        let mut providers = std::collections::HashSet::new();
        let mut runtime = "RUST_WASM".to_string();

        // Empty requirements
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        // Match any node_id
        reqs.node_id_match_any.push(node_id);
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        reqs.node_id_match_any.push(uuid::Uuid::new_v4());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        reqs.node_id_match_any.clear();
        reqs.node_id_match_any.push(uuid::Uuid::new_v4());
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
        reqs.node_id_match_any.clear();

        // Match all labels
        reqs.label_match_all.push("red".to_string());
        caps.labels.push("green".to_string());
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        caps.labels.push("red".to_string());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        reqs.label_match_all.push("blue".to_string());
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        caps.labels.push("blue".to_string());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        // Match all providers
        reqs.resource_match_all.push("file-1".to_string());
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        providers.insert("file-1".to_string());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        providers.insert("file-2".to_string());
        providers.insert("file-3".to_string());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        reqs.resource_match_all.push("file-9".to_string());
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        providers.insert("file-9".to_string());
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        // Match TEE and TPM
        reqs.tee = AffinityLevel::Required;
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
        caps.is_tee_running = true;
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        reqs.tpm = AffinityLevel::Required;
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
        caps.has_tpm = true;
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));

        // Match runtime
        runtime = "CONTAINER".to_string();
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
        runtime = "".to_string();
        assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
        runtime = "RUST_WASM".to_string();
        assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
            &runtime, &reqs, &node_id, &caps, &providers
        ));
    }
}
