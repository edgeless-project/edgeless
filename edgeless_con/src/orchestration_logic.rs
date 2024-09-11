// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

// Originally copied over from edgeless_orc.

use rand::distributions::Distribution;
use rand::SeedableRng;

/// Keeps all the necessary state that is needed to make simple orchestration
/// decisions. Provides convenience methods that can be used by the
/// orchestrator.
pub struct OrchestrationLogic {
    /// Orchestration strategy.
    orchestration_strategy: crate::orchestration_utils::OrchestrationStrategy,
    /// Random-number generator.
    rng: rand::rngs::StdRng,
}

impl OrchestrationLogic {
    pub fn new(orchestration_strategy: crate::orchestration_utils::OrchestrationStrategy) -> Self {
        match orchestration_strategy {
            crate::orchestration_utils::OrchestrationStrategy::Random => log::info!("Orchestration logic strategy: random"),
        };

        Self {
            orchestration_strategy,
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    /// Return true if it is possible to assign a function requesting a given
    /// run-time and with given deployment requirements to a node with
    /// given UUID and capabilities.
    pub fn is_node_feasible(
        runtime: &str,
        reqs: &crate::orchestration_utils::DeploymentRequirements,
        node_id: &uuid::Uuid,
        capabilities: &edgeless_api::node_registration::NodeCapabilities,
        resource_providers: &std::collections::HashMap<String, crate::controller::server::ResourceProvider>,
    ) -> bool {
        if !capabilities.runtimes.iter().any(|x| x == runtime) {
            return false;
        }
        if !reqs.node_id_match_any.is_empty() && !reqs.node_id_match_any.contains(node_id) {
            return false;
        }
        for label in reqs.label_match_all.iter() {
            if !capabilities.labels.contains(label) {
                return false;
            }
        }
        for provider in reqs.resource_match_all.iter() {
            if resource_providers.get(provider).is_none() {
                return false;
            }
        }
        match reqs.tee {
            crate::orchestration_utils::AffinityLevel::Required => {
                if !capabilities.is_tee_running {
                    return false;
                }
            }
            crate::orchestration_utils::AffinityLevel::NotRequired => {}
        }
        match reqs.tpm {
            crate::orchestration_utils::AffinityLevel::Required => {
                if !capabilities.has_tpm {
                    return false;
                }
            }
            crate::orchestration_utils::AffinityLevel::NotRequired => {}
        }
        true
    }

    /// Select the next node on which a function instance should be spawned,
    /// based on a general orchestration strategy as defined in the settings.
    /// Always match the deployment requirements specified with the nodes'
    /// capabilities.
    pub fn next(
        &mut self,
        node_pool: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        spawn_req: &edgeless_api::workflow_instance::WorkflowFunction,
    ) -> Option<uuid::Uuid> {
        if node_pool.is_empty() {
            return None;
        }
        let reqs = crate::orchestration_utils::DeploymentRequirements::from_annotations(&spawn_req.annotations);
        match self.orchestration_strategy {
            crate::orchestration_utils::OrchestrationStrategy::Random => {
                // Select only the nodes that are feasible.
                let mut candidates = vec![];
                let mut high: f32 = 0.0;
                for (node_id, node_desc) in node_pool {
                    if Self::is_node_feasible(
                        &spawn_req.function_class_specification.function_class_type,
                        &reqs,
                        &node_id,
                        &node_desc.capabilities,
                        &node_desc.resource_providers,
                    ) {
                        candidates.push((node_id.clone(), node_desc.weight));
                        high += &node_desc.weight;
                    }
                }
                if high > 0.0 {
                    let rv = rand::distributions::Uniform::new(0.0, high);
                    let rnd = rv.sample(&mut self.rng);
                    let mut sum = 0.0_f32;
                    for (node_id, node_weight) in candidates {
                        sum += node_weight;
                        if sum >= rnd {
                            return Some(node_id);
                        }
                    }
                }
                None
            }
        }
    }
}
