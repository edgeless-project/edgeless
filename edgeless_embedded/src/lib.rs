// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
#![no_std]

extern crate alloc;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

const REGISTRATION_PEER: smoltcp::wire::IpEndpoint = smoltcp::wire::IpEndpoint {
    addr: embassy_net::IpAddress::v4(192, 168, 101, 2),
    port: 7050,
};

pub mod agent;
pub mod coap;
pub mod dataplane;
pub mod invocation;
pub mod resource;
pub mod resource_configuration;
