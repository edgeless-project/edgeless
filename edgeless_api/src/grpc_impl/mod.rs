// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod api {
    tonic::include_proto!("edgeless_api");
}
mod common;
mod inner;
// only the outer grpc_impl is exported; inner stays private to this crate; only use the outer traits in your code
pub mod outer;
