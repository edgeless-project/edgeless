// Rebalancer based on the one developed by CNR: https://github.com/edgeless-project/cnr-experiments/blob/main/delegated_orc/src/rebalancer.rs

use edgeless_api::function_instance::{ComponentId, NodeId};
use edgeless_api::node_registration::{NodeCapabilities, NodeHealthStatus};
use edgeless_orc::proxy::Proxy;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

struct NodeDesc {
    function_instances: Vec<ComponentId>,
    capabilities: NodeCapabilities,
    resource_providers: HashSet<String>,
    pub health_status: Option<NodeHealthStatus>,
    fair_share: f64,
}

impl NodeDesc {
    fn credit(&self) -> f64 {
        self.function_instances.len() as f64 - self.fair_share
    }

    pub fn memory_usage_percent(&self) -> Option<f64> {
        self.health_status.as_ref().and_then(|health| {
            let total_memory = health.mem_used + health.mem_available;
            if total_memory > 0 {
                Some((health.mem_used as f64 * 100.0) / total_memory as f64)
            } else {
                None
            }
        })
    }

    pub fn cpu_usage_percent(&self) -> Option<f64> {
        self.health_status.as_ref().map(|health| health.proc_cpu_usage as f64)
    }
}

struct InstanceDesc {
    runtime: String,
    deployment_requirements: edgeless_orc::deployment_requirements::DeploymentRequirements,
}

pub struct Rebalancer {
    proxy: edgeless_orc::proxy_redis::ProxyRedis,
    nodes: HashMap<NodeId, NodeDesc>,
    instances: HashMap<ComponentId, InstanceDesc>,
}

impl Rebalancer {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        let proxy = match edgeless_orc::proxy_redis::ProxyRedis::new(redis_url, false, None) {
            Ok(proxy) => proxy,
            Err(err) => anyhow::bail!("Could not connect to Redis at {}: {}", redis_url, err),
        };
        Ok(Self {
            proxy,
            nodes: HashMap::new(),
            instances: HashMap::new(),
        })
    }

    pub fn update_state(&mut self) -> HashSet<String> {
        self.nodes.clear();
        self.instances.clear();

        let node_capabilities = self.proxy.fetch_node_capabilities();
        let health_data = self.proxy.fetch_node_health();
        let active_node_ids: HashSet<String> = node_capabilities.keys().map(|id| id.to_string()).collect();

        for (node_id, capabilities) in node_capabilities {
            self.nodes.insert(
                node_id,
                NodeDesc {
                    function_instances: vec![],
                    capabilities,
                    resource_providers: HashSet::new(),
                    health_status: health_data.get(&node_id).cloned(),
                    fair_share: 0.0,
                },
            );
        }

        let mut instances_map = self.proxy.fetch_nodes_to_instances();
        for (node_id, instances) in &mut instances_map {
            if let Some(node) = self.nodes.get_mut(node_id) {
                for instance in instances {
                    if let edgeless_orc::proxy::Instance::Function(lid) = instance {
                        node.function_instances.push(*lid);
                    }
                }
            }
        }

        let providers = self.proxy.fetch_resource_providers();
        for (provider_id, resource_provider) in providers {
            if let Some(node) = self.nodes.get_mut(&resource_provider.node_id) {
                node.resource_providers.insert(provider_id);
            }
        }

        self.assign_fair_share();

        active_node_ids
    }

    fn assign_fair_share(&mut self) {
        let mut fair_shares = HashMap::new();
        for node_id in self.nodes.keys() {
            fair_shares.insert(*node_id, 0.0);
        }

        let mut instances = self.proxy.fetch_function_instance_requests();
        for (lid, req) in &mut instances {
            let runtime = req.spec.function_type.clone();
            let deployment_requirements = edgeless_orc::deployment_requirements::DeploymentRequirements::from_annotations(&req.annotations);

            let feasible_nodes: Vec<_> = self
                .nodes
                .iter()
                .filter(|(node_id, node_desc)| {
                    edgeless_orc::orchestration_logic::OrchestrationLogic::is_node_feasible(
                        &runtime,
                        &deployment_requirements,
                        node_id,
                        &node_desc.capabilities,
                        &node_desc.resource_providers,
                    )
                })
                .map(|(node_id, _)| *node_id)
                .collect();

            self.instances.insert(
                *lid,
                InstanceDesc {
                    runtime,
                    deployment_requirements,
                },
            );

            if !feasible_nodes.is_empty() {
                let share = 1.0 / feasible_nodes.len() as f64;
                for node_id in feasible_nodes {
                    if let Some(fair_share) = fair_shares.get_mut(&node_id) {
                        *fair_share += share;
                    }
                }
            }
        }

        for (node_id, fair_share) in fair_shares {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.fair_share = fair_share;
            }
        }
    }

    pub fn rebalance_cluster(&mut self) -> usize {
        let mut credits: HashMap<_, _> = self.nodes.iter().map(|(id, desc)| (*id, desc.credit())).collect();
        let mut migrations = vec![];

        for (node_id, node_desc) in &self.nodes {
            for lid in &node_desc.function_instances {
                if credits[node_id] <= 0.0 {
                    break;
                }

                let instance_desc = self.instances.get(lid).unwrap();

                for (target_node_id, target_node_desc) in &self.nodes {
                    if credits[target_node_id] < 0.0
                        && edgeless_orc::orchestration_logic::OrchestrationLogic::is_node_feasible(
                            &instance_desc.runtime,
                            &instance_desc.deployment_requirements,
                            target_node_id,
                            &target_node_desc.capabilities,
                            &target_node_desc.resource_providers,
                        )
                    {
                        migrations.push(edgeless_orc::deploy_intent::DeployIntent::Migrate(*lid, vec![*target_node_id]));
                        *credits.get_mut(node_id).unwrap() -= 1.0;
                        *credits.get_mut(target_node_id).unwrap() += 1.0;
                        break;
                    }
                }
            }
        }

        let num_migrations = migrations.len();
        if num_migrations > 0 {
            self.proxy.add_deploy_intents(migrations);
            log::info!("Rebalancing cluster: triggered {} migrations.", num_migrations);
        }
        num_migrations
    }

    pub fn should_create_node(&self, credit_threshold: f64, cpu_threshold: f64, mem_threshold: f64) -> bool {
        // Option 1: Relative overload across the cluster
        let total_overload: f64 = self.nodes.values().map(|n| n.credit()).filter(|c| *c > 0.0).sum();
        if total_overload > credit_threshold {
            log::warn!(
                "SCALE-UP decision: Relative overload detected (credit sum {} > {})",
                total_overload,
                credit_threshold
            );
            return true;
        }

        // Option 2: Absolute saturation of individual nodes (CPU or memory)
        for (id, node) in &self.nodes {
            if let Some(cpu_usage) = node.cpu_usage_percent() {
                if cpu_usage > cpu_threshold {
                    log::warn!(
                        "SCALE-UP decision: Absolute CPU saturation on node {} ({}% > {}%)",
                        id,
                        cpu_usage,
                        cpu_threshold
                    );
                    return true;
                }
            }
            if let Some(mem_usage) = node.memory_usage_percent() {
                if mem_usage > mem_threshold {
                    log::warn!(
                        "SCALE-UP decision: Absolute Memory saturation on node {} ({}% > {}%)",
                        id,
                        mem_usage,
                        mem_threshold
                    );
                    return true;
                }
            }
        }

        false
    }

    pub fn empty_node(&mut self, node_to_empty_id: &str) -> usize {
        let node_id_uuid = if let Ok(uuid) = Uuid::parse_str(node_to_empty_id) {
            uuid
        } else {
            log::warn!("Invalid UUID format for node_to_empty_id: {}", node_to_empty_id);
            return 0;
        };

        let node_to_empty = if let Some(node) = self.nodes.get(&node_id_uuid) {
            node
        } else {
            log::warn!("Cannot empty node {}: not found in current state.", node_to_empty_id);
            return 0;
        };

        let mut migrations = vec![];
        for lid in &node_to_empty.function_instances {
            let instance_desc = self.instances.get(lid).unwrap();
            for (target_node_id, target_node_desc) in &self.nodes {
                if target_node_id != &node_id_uuid
                    && edgeless_orc::orchestration_logic::OrchestrationLogic::is_node_feasible(
                        &instance_desc.runtime,
                        &instance_desc.deployment_requirements,
                        target_node_id,
                        &target_node_desc.capabilities,
                        &target_node_desc.resource_providers,
                    )
                {
                    migrations.push(edgeless_orc::deploy_intent::DeployIntent::Migrate(*lid, vec![*target_node_id]));
                    break;
                }
            }
        }

        let num_migrations = migrations.len();
        if num_migrations > 0 {
            self.proxy.add_deploy_intents(migrations);
            log::info!("Attempting to empty node {}: triggered {} migrations.", node_to_empty_id, num_migrations);
        }
        num_migrations
    }

    pub fn cordon_node(&mut self, node_to_cordon_id: &str) -> bool {
        let node_id_uuid = if let Ok(uuid) = Uuid::parse_str(node_to_cordon_id) {
            uuid
        } else {
            log::warn!("Invalid UUID format for node_to_cordon_id: {}", node_to_cordon_id);
            return false;
        };

        if !self.nodes.contains_key(&node_id_uuid) {
            return false;
        };

        self.proxy
            .add_deploy_intents(vec![edgeless_orc::deploy_intent::DeployIntent::Cordon(node_id_uuid)]);
        log::info!("Attempting to cordon node {}", node_to_cordon_id);

        true
    }

    pub fn find_node_to_delete(&self, cloud_node_ids: &HashSet<String>, cpu_threshold: f64, mem_threshold: f64) -> Option<String> {
        for (node_id, node_desc) in &self.nodes {
            let node_id_str = node_id.to_string();

            if !cloud_node_ids.contains(&node_id_str) {
                continue;
            }

            let is_cpu_low = node_desc.cpu_usage_percent().is_some_and(|cpu| cpu < cpu_threshold);
            let is_mem_low = node_desc.memory_usage_percent().is_some_and(|mem| mem < mem_threshold);

            if is_cpu_low && is_mem_low {
                log::warn!(
                    "SCALE-DOWN decision: Node {} is empty and underutilized (CPU < {}%, Mem < {}%). Targeting for deletion.",
                    node_id_str,
                    cpu_threshold,
                    mem_threshold
                );
                return Some(node_id_str);
            }
        }

        None
    }

    pub fn is_node_empty(&self, node_id: &str) -> bool {
        if let Ok(uuid) = Uuid::parse_str(node_id) {
            self.nodes.get(&uuid).is_none_or(|node| node.function_instances.is_empty())
        } else {
            true
        }
    }
}
