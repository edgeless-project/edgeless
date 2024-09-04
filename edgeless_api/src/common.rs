// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_api_core::instance_id::{ComponentId, InstanceId};

#[derive(Clone, Debug, PartialEq)]
pub struct ResponseError {
    pub summary: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StartComponentResponse<InstanceIdType> {
    ResponseError(crate::common::ResponseError),
    InstanceId(InstanceIdType),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    Single(InstanceId, crate::function_instance::PortId),
    Any(Vec<(InstanceId, crate::function_instance::PortId)>),
    All(Vec<(InstanceId, crate::function_instance::PortId)>),
    Link(crate::link::LinkInstanceId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    Link(crate::link::LinkInstanceId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatchRequest {
    pub function_id: ComponentId,
    pub output_mapping: std::collections::HashMap<String, Output>,
    pub input_mapping: std::collections::HashMap<String, Input>,
}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &self.detail {
            Some(detail) => write!(fmt, "{} [detail: {}]", self.summary, detail),
            None => write!(fmt, "{}", self.summary),
        }
    }
}
