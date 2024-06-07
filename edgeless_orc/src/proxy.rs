// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[async_trait::async_trait]
pub trait Proxy: Sync + Send {
    /// Update the info on the currently actives nodes as given.
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>);
}
