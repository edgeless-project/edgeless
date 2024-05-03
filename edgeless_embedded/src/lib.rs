// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");
const COAP_PEERS: [(uuid::Uuid, smoltcp::wire::IpEndpoint); 1] = [(
    uuid::uuid!("fda6ce79-46df-4f96-a0d2-456f720f606c"),
    smoltcp::wire::IpEndpoint {
        addr: embassy_net::IpAddress::v4(192, 168, 101, 2),
        port: 7002,
    },
)];

pub mod agent;
pub mod coap;
pub mod dataplane;
pub mod invocation;
pub mod resource;
pub mod resource_configuration;
