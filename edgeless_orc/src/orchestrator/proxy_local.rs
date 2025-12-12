// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Simplified node description for ProxyLocal that doesn't include API connections
#[derive(Clone)]
struct NodeDesc {
    capabilities: edgeless_api::node_registration::NodeCapabilities,
}

impl From<&crate::client_desc::ClientDesc> for NodeDesc {
    fn from(client_desc: &crate::client_desc::ClientDesc) -> Self {
        NodeDesc {
            capabilities: client_desc.capabilities.clone(),
        }
    }
}

/// A local in-memory orchestrator proxy for testing.
/// Keeps all orchestrator state in memory without external dependencies.
#[derive(Default)]
pub struct ProxyLocal {
    // Deployment intents
    intents: Vec<crate::deploy_intent::DeployIntent>,
    
    // Node information
    nodes: std::collections::HashMap<uuid::Uuid, NodeDesc>,
    resource_providers: std::collections::HashMap<String, crate::resource_provider::ResourceProvider>,
    
    // Active instances and dependencies
    active_instances: std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>,
    dependency_graph: std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
    
    // Domain info
    domain_info: crate::domain_info::DomainInfo,
    
    // Health and performance data
    node_health: std::collections::HashMap<uuid::Uuid, edgeless_api::node_registration::NodeHealthStatus>,
    node_health_history: std::collections::HashMap<uuid::Uuid, Vec<(chrono::DateTime<chrono::Utc>, edgeless_api::node_registration::NodeHealthStatus)>>,
    performance_samples: std::collections::HashMap<String, std::collections::HashMap<String, Vec<(chrono::DateTime<chrono::Utc>, String)>>>,
}

impl ProxyLocal {
    pub fn new() -> Self {
        Self::default()
    }
}

impl crate::proxy::Proxy for ProxyLocal {
    fn add_deploy_intents(&mut self, intents: Vec<crate::deploy_intent::DeployIntent>) {
        self.intents.append(&mut intents.clone());
    }
    
    fn retrieve_deploy_intents(&mut self) -> Vec<crate::deploy_intent::DeployIntent> {
        std::mem::take(&mut self.intents)
    }

    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>) {
        self.nodes.clear();
        for (node_id, desc) in nodes {
            self.nodes.insert(*node_id, NodeDesc::from(desc));
        }
    }
    
    fn update_resource_providers(&mut self, resource_providers: &std::collections::HashMap<String, crate::resource_provider::ResourceProvider>) {
        self.resource_providers = resource_providers.clone();
    }
    
    fn update_active_instances(&mut self, active_instances: &std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>) {
        self.active_instances = active_instances.clone();
    }
    
    fn update_dependency_graph(&mut self, dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {
        self.dependency_graph = dependency_graph.clone();
    }
    
    fn update_domain_info(&mut self, domain_info: &crate::domain_info::DomainInfo) {
        self.domain_info = domain_info.clone();
    }
    
    fn push_node_health(&mut self, node_id: &uuid::Uuid, node_health: edgeless_api::node_registration::NodeHealthStatus) {
        // Update the latest health status
        self.node_health.insert(*node_id, node_health.clone());
        
        // Add to history with timestamp
        let timestamp = chrono::Utc::now();
        self.node_health_history
            .entry(*node_id)
            .or_insert_with(Vec::new)
            .push((timestamp, node_health));
    }
    
    fn push_performance_samples(&mut self, _node_id: &uuid::Uuid, performance_samples: edgeless_api::node_registration::NodePerformanceSamples) {
        // Process function execution times
        for (function_id, samples) in &performance_samples.function_execution_times {
            let entry = self.performance_samples
                .entry(function_id.to_string())
                .or_insert_with(std::collections::HashMap::new);
            
            let metric_entry = entry
                .entry("function_execution_time".to_string())
                .or_insert_with(Vec::new);
            
            for sample in samples {
                let timestamp = chrono::DateTime::from_timestamp(sample.timestamp_sec, sample.timestamp_ns)
                    .unwrap_or_else(chrono::Utc::now);
                metric_entry.push((timestamp, sample.sample.to_string()));
            }
        }
        
        // Process function transfer times
        for (function_id, samples) in &performance_samples.function_transfer_times {
            let entry = self.performance_samples
                .entry(function_id.to_string())
                .or_insert_with(std::collections::HashMap::new);
            
            let metric_entry = entry
                .entry("function_transfer_time".to_string())
                .or_insert_with(Vec::new);
            
            for sample in samples {
                let timestamp = chrono::DateTime::from_timestamp(sample.timestamp_sec, sample.timestamp_ns)
                    .unwrap_or_else(chrono::Utc::now);
                metric_entry.push((timestamp, sample.sample.to_string()));
            }
        }
        
        // Process function log entries
        for (function_id, log_entries) in &performance_samples.function_log_entries {
            let entry = self.performance_samples
                .entry(function_id.to_string())
                .or_insert_with(std::collections::HashMap::new);
            
            let metric_entry = entry
                .entry(log_entries.first().map(|e| e.target.clone()).unwrap_or_else(|| "unknown".to_string()))
                .or_insert_with(Vec::new);
            
            for log_entry in log_entries {
                let timestamp = chrono::DateTime::from_timestamp(log_entry.timestamp_sec, log_entry.timestamp_ns)
                    .unwrap_or_else(chrono::Utc::now);
                metric_entry.push((timestamp, log_entry.message.clone()));
            }
        }
    }
    
    fn fetch_domain_info(&mut self) -> crate::domain_info::DomainInfo {
        self.domain_info.clone()
    }
    
    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities> {
        self.nodes.iter().map(|(node_id, desc)| (*node_id, desc.capabilities.clone())).collect()
    }
    
    fn fetch_resource_providers(&mut self) -> std::collections::HashMap<String, crate::resource_provider::ResourceProvider> {
        self.resource_providers.clone()
    }
    
    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeHealthStatus> {
        self.node_health.clone()
    }
    
    fn fetch_node_healths(&mut self) -> crate::proxy::NodeHealthStatuses {
        self.node_health_history.clone()
    }
    
    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, crate::proxy::PerformanceSamples> {
        self.performance_samples.clone()
    }
    
    fn fetch_function_instance_requests(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::SpawnFunctionRequest> {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            if let crate::active_instance::ActiveInstance::Function(spawn_req, _) = instance {
                result.insert(*logical_id, spawn_req.clone());
            }
        }
        result
    }
    
    fn fetch_resource_instance_configurations(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::resource_configuration::ResourceInstanceSpecification>
    {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            if let crate::active_instance::ActiveInstance::Resource(spec, _) = instance {
                result.insert(*logical_id, spec.clone());
            }
        }
        result
    }
    
    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<(edgeless_api::function_instance::NodeId, bool)>> {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            if let crate::active_instance::ActiveInstance::Function(_, instance_ids) = instance {
                result.insert(*logical_id, instance_ids.iter().map(|x| (x.0.node_id, x.1)).collect());
            }
        }
        result
    }
    
    fn fetch_instances_to_physical_ids(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<(edgeless_api::function_instance::ComponentId, bool)>> {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            match instance {
                crate::active_instance::ActiveInstance::Function(_, instance_ids) => {
                    result.insert(*logical_id, instance_ids.iter().map(|x| (x.0.function_id, x.1)).collect());
                }
                crate::active_instance::ActiveInstance::Resource(_, instance_id) => {
                    result.insert(*logical_id, vec![(instance_id.function_id, true)]);
                }
            }
        }
        result
    }
    
    fn fetch_resource_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::NodeId> {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            if let crate::active_instance::ActiveInstance::Resource(_, instance_id) = instance {
                result.insert(*logical_id, instance_id.node_id);
            }
        }
        result
    }
    
    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<crate::proxy::Instance>> {
        let mut result = std::collections::HashMap::new();
        for (logical_id, instance) in &self.active_instances {
            match instance {
                crate::active_instance::ActiveInstance::Function(_, instance_ids) => {
                    for (instance_id, _is_active) in instance_ids {
                        result
                            .entry(instance_id.node_id)
                            .or_insert_with(Vec::new)
                            .push(crate::proxy::Instance::Function(*logical_id));
                    }
                }
                crate::active_instance::ActiveInstance::Resource(_, instance_id) => {
                    result
                        .entry(instance_id.node_id)
                        .or_insert_with(Vec::new)
                        .push(crate::proxy::Instance::Resource(*logical_id));
                }
            }
        }
        result
    }
    
    fn fetch_dependency_graph(&mut self) -> std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>> {
        self.dependency_graph.clone()
    }
    
    fn fetch_logical_id_to_workflow_id(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, String> {
        self.active_instances
            .iter()
            .map(|(logical_id, instance)| (*logical_id, instance.workflow_id()))
            .collect()
    }
    
    fn updated(&mut self, _category: crate::proxy::Category) -> bool {
        // Always return true for local proxy since we're always in sync
        true
    }
    
    fn garbage_collection(&mut self, period: tokio::time::Duration) {
        let cutoff_time = chrono::Utc::now() - chrono::Duration::from_std(period).unwrap_or(chrono::Duration::zero());
        
        // Clean old health history
        for (_node_id, history) in self.node_health_history.iter_mut() {
            history.retain(|(timestamp, _)| *timestamp > cutoff_time);
        }
        
        // Clean old performance samples
        for (_function_id, metrics) in self.performance_samples.iter_mut() {
            for (_metric_name, samples) in metrics.iter_mut() {
                samples.retain(|(timestamp, _)| *timestamp > cutoff_time);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::Proxy;

    #[test]
    fn test_proxy_local_basic() {
        let mut proxy = ProxyLocal::new();
        
        // Test domain info
        let domain_info = crate::domain_info::DomainInfo {
            domain_id: "test-domain".to_string(),
        };
        proxy.update_domain_info(&domain_info);
        assert_eq!(proxy.fetch_domain_info().domain_id, "test-domain");
        
        // Test intents
        let intents = vec![
            crate::deploy_intent::DeployIntent::Migrate(uuid::Uuid::new_v4(), vec![uuid::Uuid::new_v4()]),
        ];
        proxy.add_deploy_intents(intents.clone());
        let retrieved = proxy.retrieve_deploy_intents();
        assert_eq!(retrieved.len(), 1);
        
        // Test that intents are consumed
        let retrieved_again = proxy.retrieve_deploy_intents();
        assert_eq!(retrieved_again.len(), 0);
    }

    #[test]
    fn test_proxy_local_active_instances() {
        let mut proxy = ProxyLocal::new();
        
        let node_id_1 = uuid::Uuid::new_v4();
        let node_id_2 = uuid::Uuid::new_v4();
        let logical_id = uuid::Uuid::new_v4();
        let physical_id_1 = uuid::Uuid::new_v4();
        let physical_id_2 = uuid::Uuid::new_v4();
        
        // Create a function with one active and one standby instance
        let mut active_instances = std::collections::HashMap::new();
        active_instances.insert(
            logical_id,
            crate::active_instance::ActiveInstance::Function(
                edgeless_api::function_instance::SpawnFunctionRequest {
                    spec: edgeless_api::function_instance::FunctionClassSpecification {
                        id: "test-func".to_string(),
                        function_type: "RUST_WASM".to_string(),
                        version: "1.0".to_string(),
                        binary: None,
                        code: None,
                        outputs: vec!["out".to_string()],
                    },
                    annotations: std::collections::HashMap::new(),
                    state_specification: edgeless_api::function_instance::StateSpecification {
                        state_id: uuid::Uuid::new_v4(),
                        state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                    },
                    workflow_id: "test-workflow".to_string(),
                    replication_factor: Some(2),
                },
                vec![
                    (edgeless_api::function_instance::InstanceId {
                        node_id: node_id_1,
                        function_id: physical_id_1,
                    }, true),  // active
                    (edgeless_api::function_instance::InstanceId {
                        node_id: node_id_2,
                        function_id: physical_id_2,
                    }, false), // standby
                ],
            ),
        );
        
        proxy.update_active_instances(&active_instances);
        
        // Test fetch_function_instances_to_nodes
        let instances_to_nodes = proxy.fetch_function_instances_to_nodes();
        assert_eq!(instances_to_nodes.len(), 1);
        let nodes = instances_to_nodes.get(&logical_id).unwrap();
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].0, node_id_1);
        assert!(nodes[0].1); // active
        assert_eq!(nodes[1].0, node_id_2);
        assert!(!nodes[1].1); // standby
        
        // Test fetch_instances_to_physical_ids
        let logical_to_physical = proxy.fetch_instances_to_physical_ids();
        assert_eq!(logical_to_physical.len(), 1);
        let physical_ids = logical_to_physical.get(&logical_id).unwrap();
        assert_eq!(physical_ids.len(), 2);
        assert_eq!(physical_ids[0].0, physical_id_1);
        assert!(physical_ids[0].1); // active
        assert_eq!(physical_ids[1].0, physical_id_2);
        assert!(!physical_ids[1].1); // standby
        
        // Test fetch_logical_id_to_workflow_id
        let workflow_mapping = proxy.fetch_logical_id_to_workflow_id();
        assert_eq!(workflow_mapping.get(&logical_id).unwrap(), "test-workflow");
    }

    #[test]
    fn test_proxy_local_health_and_performance() {
        let mut proxy = ProxyLocal::new();
        let node_id = uuid::Uuid::new_v4();
        
        // Push some health data
        let health1 = edgeless_api::node_registration::NodeHealthStatus {
            mem_free: 1000,
            mem_used: 2000,
            mem_available: 3000,
            proc_cpu_usage: 50,
            proc_memory: 100,
            proc_vmemory: 200,
            load_avg_1: 1,
            load_avg_5: 2,
            load_avg_15: 3,
            tot_rx_bytes: 1000,
            tot_rx_pkts: 10,
            tot_rx_errs: 0,
            tot_tx_bytes: 2000,
            tot_tx_pkts: 20,
            tot_tx_errs: 0,
            disk_free_space: 10000,
            disk_tot_reads: 100,
            disk_tot_writes: 200,
            gpu_load_perc: 0,
            gpu_temp_cels: 0,
            active_power: 50,
        };
        
        proxy.push_node_health(&node_id, health1.clone());
        
        // Fetch latest health
        let latest_health = proxy.fetch_node_health();
        assert_eq!(latest_health.len(), 1);
        assert_eq!(latest_health.get(&node_id).unwrap().mem_free, 1000);
        
        // Fetch health history
        let health_history = proxy.fetch_node_healths();
        assert_eq!(health_history.len(), 1);
        assert_eq!(health_history.get(&node_id).unwrap().len(), 1);
        
        // Push performance samples
        let function_id = uuid::Uuid::new_v4();
        let samples = edgeless_api::node_registration::NodePerformanceSamples {
            function_execution_times: std::collections::HashMap::from([(
                function_id,
                vec![edgeless_api::node_registration::Sample {
                    timestamp_sec: 1000,
                    timestamp_ns: 0,
                    sample: 42.0,
                }],
            )]),
            function_transfer_times: std::collections::HashMap::new(),
            function_log_entries: std::collections::HashMap::new(),
        };
        
        proxy.push_performance_samples(&node_id, samples);
        
        let perf_samples = proxy.fetch_performance_samples();
        assert_eq!(perf_samples.len(), 1);
        let func_samples = perf_samples.get(&function_id.to_string()).unwrap();
        assert!(func_samples.contains_key("function_execution_time"));
    }

    #[test]
    fn test_proxy_local_garbage_collection() {
        let mut proxy = ProxyLocal::new();
        let node_id = uuid::Uuid::new_v4();
        
        // Push health data
        let health = edgeless_api::node_registration::NodeHealthStatus {
            mem_free: 1000,
            mem_used: 2000,
            mem_available: 3000,
            proc_cpu_usage: 50,
            proc_memory: 100,
            proc_vmemory: 200,
            load_avg_1: 1,
            load_avg_5: 2,
            load_avg_15: 3,
            tot_rx_bytes: 1000,
            tot_rx_pkts: 10,
            tot_rx_errs: 0,
            tot_tx_bytes: 2000,
            tot_tx_pkts: 20,
            tot_tx_errs: 0,
            disk_free_space: 10000,
            disk_tot_reads: 100,
            disk_tot_writes: 200,
            gpu_load_perc: 0,
            gpu_temp_cels: 0,
            active_power: 50,
        };
        proxy.push_node_health(&node_id, health);
        
        // Verify data exists
        let health_history = proxy.fetch_node_healths();
        assert_eq!(health_history.get(&node_id).unwrap().len(), 1);
        
        // Run garbage collection with 0 period (should remove everything)
        proxy.garbage_collection(tokio::time::Duration::from_secs(0));
        
        // Verify old data is removed
        let health_history_after = proxy.fetch_node_healths();
        assert_eq!(health_history_after.get(&node_id).unwrap().len(), 0);
    }
}
