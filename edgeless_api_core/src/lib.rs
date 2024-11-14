// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#![no_std]

// #[cfg(feature = "embedded")]
// #![feature(async_fn_in_trait)]

extern crate alloc;

pub mod coap_mapping;
pub mod common;
pub mod instance_id;
pub mod invocation;
pub mod node_registration;
pub mod resource_configuration;
