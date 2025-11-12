// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// An orchestrator proxy that does nothing.
pub struct ProxyNone {}

impl super::proxy::Proxy for ProxyNone {
    fn update_nodes(&mut self, _nodes: &std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>) {}
    fn update_resource_providers(&mut self, _resource_providers: &std::collections::HashMap<String, crate::resource_provider::ResourceProvider>) {}
    fn update_active_instances(&mut self, _active_instances: &std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>) {}
    fn update_dependency_graph(&mut self, _dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {}
    fn update_domain_info(&mut self, _domain_info: &crate::domain_info::DomainInfo) {}
    fn push_node_health(&mut self, _node_id: &uuid::Uuid, _node_health: edgeless_api::node_registration::NodeHealthStatus) {}
    fn push_performance_samples(&mut self, _node_id: &uuid::Uuid, _performance_samples: edgeless_api::node_registration::NodePerformanceSamples) {}
    fn add_deploy_intents(&mut self, _intents: Vec<crate::deploy_intent::DeployIntent>) {}
    fn retrieve_deploy_intents(&mut self) -> Vec<crate::deploy_intent::DeployIntent> {
        vec![]
    }
    fn fetch_domain_info(&mut self) -> crate::domain_info::DomainInfo {
        crate::domain_info::DomainInfo::default()
    }
    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities> {
        std::collections::HashMap::new()
    }
    fn fetch_resource_providers(&mut self) -> std::collections::HashMap<String, crate::resource_provider::ResourceProvider> {
        std::collections::HashMap::new()
    }
    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeHealthStatus> {
        std::collections::HashMap::new()
    }
    fn fetch_node_healths(&mut self) -> crate::proxy::NodeHealthStatuses {
        std::collections::HashMap::new()
    }
    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, crate::proxy::PerformanceSamples> {
        std::collections::HashMap::new()
    }
    fn fetch_performance_series(&mut self, _node: &str, _series: &str) -> crate::proxy::PerformanceSeries {
        vec![]
    }
    fn fetch_function_instance_requests(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::SpawnFunctionRequest> {
        std::collections::HashMap::new()
    }
    fn fetch_resource_instance_configurations(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::resource_configuration::ResourceInstanceSpecification>
    {
        std::collections::HashMap::new()
    }
    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>> {
        std::collections::HashMap::new()
    }
    fn fetch_instances_to_physical_ids(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::ComponentId>> {
        std::collections::HashMap::new()
    }
    fn fetch_resource_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::NodeId> {
        std::collections::HashMap::new()
    }
    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<crate::proxy::Instance>> {
        std::collections::HashMap::new()
    }
    fn fetch_dependency_graph(&mut self) -> std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>> {
        std::collections::HashMap::new()
    }
    fn fetch_logical_id_to_workflow_id(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, String> {
        std::collections::HashMap::new()
    }
    fn updated(&mut self, _category: crate::proxy::Category) -> bool {
        true
    }
    fn garbage_collection(&mut self, _period: tokio::time::Duration) {}
}
