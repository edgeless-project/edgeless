// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct ClientDesc {
    pub agent_url: String,
    pub invocation_url: String,
    pub api: Box<dyn edgeless_api::outer::agent::AgentAPI + Send>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
}
