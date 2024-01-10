// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
pub mod agent;

pub mod api {
    tonic::include_proto!("edgeless_api");
}

pub mod common;

pub mod controller;

pub mod function_instance;

pub mod invocation;

pub mod orc;

pub mod resource_configuration;

pub mod workflow_instance;

pub mod node_management;

pub mod node_registration;
