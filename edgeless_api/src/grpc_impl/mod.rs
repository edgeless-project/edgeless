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
#[cfg(not(target_os = "macos"))]
pub mod tls_config_mtls;
#[cfg(target_os = "macos")]
pub mod tls_config_none;
