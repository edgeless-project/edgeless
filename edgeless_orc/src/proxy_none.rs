// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// An orchestrator proxy that does nothing.
pub struct ProxyNone {}

impl super::proxy::Proxy for ProxyNone {
    fn update_nodes(&mut self, _nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {}
}
