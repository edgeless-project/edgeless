// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// TODO: the protos should be moved to the grpc_impl
pub mod grpc_api_stubs {
    tonic::include_proto!("edgeless_api");
}
pub mod common;
pub mod domain_registration;
pub mod function_instance;
pub mod guest_api_function;
pub mod guest_api_host;
pub mod invocation;
pub mod node_management;
pub mod node_registration;
pub mod outer;
pub mod resource_configuration;
pub mod workflow_instance;
