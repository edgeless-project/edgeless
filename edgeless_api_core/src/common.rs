// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#[derive(Clone, Debug)]
pub struct ErrorResponse {
    pub summary: &'static str,
    pub detail: Option<&'static str>,
}

#[derive(Debug, Clone, minicbor::Decode, minicbor::Encode, minicbor::CborLen, PartialEq, Eq)]
pub enum Output {
    #[n(0)]
    Single(#[n(0)] crate::instance_id::InstanceId),
    #[n(1)]
    Any(#[n(0)] crate::instance_id::InstanceIdVec<4>),
    #[n(2)]
    All(#[n(0)] crate::instance_id::InstanceIdVec<4>),
}
