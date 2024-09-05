// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// An orchestrator proxy that does nothing.
pub struct ProxyNone {}

impl super::proxy::Proxy for ProxyNone {
    fn update_nodes(&mut self, _nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {}
    fn update_resource_providers(&mut self, _resource_providers: &std::collections::HashMap<String, super::orchestrator::ResourceProvider>) {}
    fn update_active_instances(&mut self, _active_instances: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ActiveInstance>) {}
    fn update_dependency_graph(&mut self, _dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {}
    fn push_keep_alive_responses(&mut self, _keep_alive_responses: Vec<(uuid::Uuid, edgeless_api::node_management::KeepAliveResponse)>) {}
    fn add_deploy_intents(&mut self, _intents: Vec<super::orchestrator::DeployIntent>) {}
    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent> {
        vec![]
    }
    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities> {
        std::collections::HashMap::new()
    }
    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_management::NodeHealthStatus> {
        std::collections::HashMap::new()
    }
    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, std::collections::HashMap<String, Vec<(f64, f64)>>> {
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
}
