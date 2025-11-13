// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod api {
    tonic::include_proto!("edgeless_api");
}
mod common;
mod inner;
pub mod outer;
pub mod tls_config;
#[cfg(target_arch = "x86_64")]
pub mod tls_config_mtls;
#[cfg(not(target_arch = "x86_64"))]
pub mod tls_config_none;

pub fn init_crypto() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
}
