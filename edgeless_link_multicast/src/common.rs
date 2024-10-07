// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MulticastConfig {
    pub ip: std::net::Ipv4Addr,
    pub port: u16,
}
