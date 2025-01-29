// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

/// Orchestration domain's public information advertised via the Proxy.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct DomainInfo {
    pub domain_id: String,
}
