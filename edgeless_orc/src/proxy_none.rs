// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// An orchestrator proxy that does nothing.
pub struct ProxyNone {}

impl super::proxy::Proxy for ProxyNone {
    fn update_nodes(&mut self, _nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {}
    fn update_resource_providers(&mut self, _resource_providers: &std::collections::HashMap<String, super::orchestrator::ResourceProvider>) {}
    fn update_active_instances(&mut self, _active_instances: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ActiveInstance>) {}
    fn update_dependency_graph(&mut self, _dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {}
    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent> {
        vec![]
    }
}
