// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use crate::orchestrator;

#[derive(Clone)]
pub enum Instance {
    Function(edgeless_api::function_instance::ComponentId),
    Resource(edgeless_api::function_instance::ComponentId),
}

#[async_trait::async_trait]
pub trait Proxy: Sync + Send {
    /// Update the info on the currently actives nodes as given.
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>);

    /// Update the info on the resource providers.
    fn update_resource_providers(&mut self, resource_providers: &std::collections::HashMap<String, super::orchestrator::ResourceProvider>);

    /// Update the active instances (functions and resources).
    fn update_active_instances(&mut self, active_instances: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ActiveInstance>);

    /// Update the dependency graph.
    fn update_dependency_graph(&mut self, dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>);

    /// Push keep-alive responses.
    fn push_keep_alive_responses(&mut self, keep_alive_responses: Vec<(uuid::Uuid, edgeless_api::node_management::KeepAliveResponse)>);

    /// Add deployment intents.
    fn add_deploy_intents(&mut self, intents: Vec<orchestrator::DeployIntent>);

    /// Retrieve the pending deploy intents. Consume the intents retrieved.
    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent>;

    /// Fetch the nodes' capabilities.
    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities>;

    /// Fetch the nodes' health status.
    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_management::NodeHealthStatus>;

    /// Fetch the performance samples.
    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, std::collections::HashMap<String, Vec<(f64, f64)>>>;

    /// Fetch the mapping between active function instances and nodes.
    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>>;

    /// Fetch the mapping between active resources instances and nodes.
    fn fetch_resource_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::NodeId>;

    /// Find all the active instances on nodes.
    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<Instance>>;
}
