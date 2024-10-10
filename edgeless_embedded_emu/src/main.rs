// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use embassy_net::{Ipv4Address, Ipv4Cidr};
use embassy_net_tuntap::TunTapDevice;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

fn main() -> ! {
    env_logger::init();

    static EXECUTOR_RAW: static_cell::StaticCell<embassy_executor::Executor> = static_cell::StaticCell::new();
    let executor = EXECUTOR_RAW.init_with(embassy_executor::Executor::new);

    executor.run(|spawner| {
        spawner.spawn(edgeless(spawner)).unwrap();
    });

    #[allow(unreachable_code, clippy::empty_loop)]
    loop {}
}

#[embassy_executor::task]
async fn net_task(stack: &'static embassy_net::Stack<TunTapDevice>) -> ! {
    stack.run().await
}

#[embassy_executor::task]
async fn registration(agent: edgeless_embedded::agent::EmbeddedAgent) {
    let mut agent = agent;
    embassy_time::Timer::after_millis(5000).await;
    log::info!("Try Register!");
    agent.register(Ipv4Address::new(192, 168, 101, 1)).await;
    log::info!("Registration done!");
}

#[embassy_executor::task]
async fn edgeless(spawner: embassy_executor::Spawner) {
    log::info!("Edgeless Embedded Async Main");

    static RX_BUF_RAW: static_cell::StaticCell<[u8; 5000]> = static_cell::StaticCell::new();
    let rx_buf = RX_BUF_RAW.init_with(|| [0_u8; 5000]);
    static RX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
    let rx_meta = RX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);
    static TX_BUF_RAW: static_cell::StaticCell<[u8; 5000]> = static_cell::StaticCell::new();
    let tx_buf = TX_BUF_RAW.init_with(|| [0_u8; 5000]);
    static TX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
    let tx_meta = TX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);

    let device = embassy_net_tuntap::TunTapDevice::new("tap0").unwrap();
    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 101, 1), 24),
        dns_servers: heapless::Vec::new(),
        gateway: None,
    });

    static STACK_RESOURCES_RAW: static_cell::StaticCell<embassy_net::StackResources<3>> = static_cell::StaticCell::new();
    static STACK_RAW: static_cell::StaticCell<embassy_net::Stack<embassy_net_tuntap::TunTapDevice>> = static_cell::StaticCell::new();
    let stack =
        STACK_RAW.init_with(|| embassy_net::Stack::new(device, config, STACK_RESOURCES_RAW.init_with(embassy_net::StackResources::<3>::new), 1234));

    spawner.spawn(net_task(stack)).unwrap();

    let sock = embassy_net::udp::UdpSocket::new(stack, rx_meta, rx_buf, tx_meta, tx_buf);

    let sensor_scd30 = edgeless_embedded::resource::mock_sensor::MockSensor::new().await;

    let display = edgeless_embedded::resource::mock_display::MockDisplay::new().await;

    static RESOURCES_RAW: static_cell::StaticCell<[&'static mut dyn edgeless_embedded::resource::ResourceDyn; 2]> = static_cell::StaticCell::new();
    let resources = RESOURCES_RAW.init_with(|| [display, sensor_scd30]);

    let resource_registry = edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID, resources).await;

    spawner
        .spawn(edgeless_embedded::coap::coap_task(
            sock,
            resource_registry.upstream_receiver().unwrap(),
            resource_registry.clone(),
        ))
        .unwrap();

    let _ = spawner.spawn(registration(resource_registry.clone()));
}
