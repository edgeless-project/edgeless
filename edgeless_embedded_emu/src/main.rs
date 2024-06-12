// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
#![feature(type_alias_impl_trait)]

use embassy_net::{Ipv4Address, Ipv4Cidr};
use embassy_net_tuntap::TunTapDevice;
use embedded_hal::delay::DelayNs;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

fn main() -> ! {
    env_logger::init();

    let executor = static_cell::make_static!(embassy_executor::Executor::new());

    executor.run(|spawner| {
        spawner.spawn(edgeless(spawner)).unwrap();
    });

    #[allow(unreachable_code)]
    loop {}
}

#[embassy_executor::task]
async fn net_task(stack: &'static embassy_net::Stack<TunTapDevice>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn registration(agent: edgeless_embedded::agent::EmbeddedAgent) {
    let mut agent = agent;
    let mut delay = embassy_time::Delay {};
    // loop {
    delay.delay_ms(10000);
    log::info!("Try Register");
    agent.register().await;
    // }
}

#[embassy_executor::task]
async fn edgeless(spawner: embassy_executor::Spawner) {
    log::info!("Edgeless Embedded Async Main");

    let rx_buf = static_cell::make_static!([0 as u8; 5000]);
    let rx_meta = static_cell::make_static!([embassy_net::udp::PacketMetadata::EMPTY; 10]);
    let tx_buf = static_cell::make_static!([0 as u8; 5000]);
    let tx_meta = static_cell::make_static!([embassy_net::udp::PacketMetadata::EMPTY; 10]);

    let device = embassy_net_tuntap::TunTapDevice::new("tap0").unwrap();
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 101, 1), 24),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });
    let stack = static_cell::make_static!(embassy_net::Stack::new(
        device,
        config,
        static_cell::make_static!(embassy_net::StackResources::<3>::new()),
        1234
    ));

    spawner.spawn(net_task(stack)).unwrap();

    let sock = embassy_net::udp::UdpSocket::new(stack, rx_meta, rx_buf, tx_meta, tx_buf);

    let sensor_scd30 = edgeless_embedded::resource::mock_sensor::MockSensor::new().await;

    let display = edgeless_embedded::resource::mock_display::MockDisplay::new().await;

    let resources = static_cell::make_static!([display, sensor_scd30]);

    let resource_registry = edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID.clone(), resources, "").await;

    spawner
        .spawn(edgeless_embedded::coap::coap_task(
            sock,
            resource_registry.upstream_receiver().unwrap(),
            resource_registry.clone(),
        ))
        .unwrap();

    spawner.spawn(registration(resource_registry.clone()));
}
