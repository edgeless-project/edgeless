// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

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

    /// Retrieve the pending deploy intents.
    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent>;
}
