// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// TODO: the protos should be moved to the grpc_impl
pub mod grpc_api_stubs {
    tonic::include_proto!("edgeless_api");
}
mod common;
mod inner;
pub mod outer;
