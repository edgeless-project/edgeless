// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub enum Instance {
    Function(edgeless_api::function_instance::ComponentId),
    Resource(edgeless_api::function_instance::ComponentId),
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub enum Category {
    NodeCapabilities,
    ResourceProviders,
    ActiveInstances,
    DependencyGraph,
}

pub type PerformanceSamples = std::collections::HashMap<String, Vec<(chrono::DateTime<chrono::Utc>, f64)>>;
pub type NodeHealthStatuses = std::collections::HashMap<
    edgeless_api::function_instance::NodeId,
    Vec<(chrono::DateTime<chrono::Utc>, edgeless_api::node_registration::NodeHealthStatus)>,
>;

#[async_trait::async_trait]
pub trait Proxy: Sync + Send {
    /// Update the info on the currently actives nodes as given.
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>);

    /// Update the info on the resource providers.
    fn update_resource_providers(&mut self, resource_providers: &std::collections::HashMap<String, crate::resource_provider::ResourceProvider>);

    /// Update the active instances (functions and resources).
    fn update_active_instances(&mut self, active_instances: &std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>);

    /// Update the dependency graph.
    fn update_dependency_graph(&mut self, dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>);

    // Update the domain's info.
    fn update_domain_info(&mut self, domain_info: &crate::domain_info::DomainInfo);

    /// Push node health status.
    fn push_node_health(&mut self, node_id: &uuid::Uuid, node_health: edgeless_api::node_registration::NodeHealthStatus);

    /// Push performance samples.
    fn push_performance_samples(&mut self, node_id: &uuid::Uuid, performance_samples: edgeless_api::node_registration::NodePerformanceSamples);

    /// Add deployment intents.
    fn add_deploy_intents(&mut self, intents: Vec<crate::deploy_intent::DeployIntent>);

    /// Retrieve the pending deploy intents. Consume the intents retrieved.
    fn retrieve_deploy_intents(&mut self) -> Vec<crate::deploy_intent::DeployIntent>;

    // Fetch the domain's info.
    fn fetch_domain_info(&mut self) -> crate::domain_info::DomainInfo;

    /// Fetch the nodes' capabilities.
    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities>;

    /// Fetch the resource providers available.
    fn fetch_resource_providers(&mut self) -> std::collections::HashMap<String, crate::resource_provider::ResourceProvider>;

    /// Fetch the last nodes' health status.
    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeHealthStatus>;

    /// Fetch all the last nodes' health statuses, with timestamp.
    fn fetch_node_healths(&mut self) -> NodeHealthStatuses;

    /// Fetch the performance samples.
    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, PerformanceSamples>;

    /// Fetch the spawn requests of active function instances.
    fn fetch_function_instance_requests(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::SpawnFunctionRequest>;

    /// Fetch the configurations of active resource instances.
    fn fetch_resource_instance_configurations(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::resource_configuration::ResourceInstanceSpecification>;

    /// Fetch the mapping between active function instances and nodes.
    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>>;

    /// Fetch the mapping between active function/resource instances and their
    /// physical identifiers.
    fn fetch_instances_to_physical_ids(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::ComponentId>>;

    /// Fetch the mapping between active resources instances and nodes.
    fn fetch_resource_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::NodeId>;

    /// Fetch all the active instances grouped by node.
    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<Instance>>;

    /// Fetch all the dependecies of logical function/resource instances.
    fn fetch_dependency_graph(&mut self) -> std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>;

    /// Fetch the mapping between logical function/resource identifiers and
    /// workflow identifiers.
    fn fetch_logical_id_to_workflow_id(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, String>;

    /// Return true if the given category has been updated since the last fetch.
    fn updated(&mut self, category: Category) -> bool;

    /// Perform a garbage collection of the sorted sets removing all values
    /// older than `period`.
    fn garbage_collection(&mut self, period: tokio::time::Duration);
}
