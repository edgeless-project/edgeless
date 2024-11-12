// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs
use embedded_svc::wifi::Wifi;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

pub async fn init(
    spawner: embassy_executor::Spawner,
    timer: hal::timer::timg::Timer<hal::timer::timg::TimerX<hal::peripherals::TIMG1>, hal::Blocking>,
    rng: hal::rng::Rng,
    radio_clock_control: hal::peripherals::RADIO_CLK,
    clocks: hal::clock::Clocks<'static>,
    radio: hal::peripherals::WIFI,
    agent: edgeless_embedded::agent::EmbeddedAgent,
) -> &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static, esp_wifi::wifi::WifiStaDevice>> {
    let init = esp_wifi::initialize(esp_wifi::EspWifiInitFor::Wifi, timer, rng.clone(), radio_clock_control, &clocks).unwrap();

    let wifi = radio;

    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(&init, wifi, esp_wifi::wifi::WifiStaDevice).unwrap();

    let net_config = embassy_net::Config::dhcpv4(Default::default());

    static STACK_RESOURCES_RAW: static_cell::StaticCell<embassy_net::StackResources<3>> = static_cell::StaticCell::new();
    static STACK_RAW: static_cell::StaticCell<embassy_net::Stack<esp_wifi::wifi::WifiDevice<'_, esp_wifi::wifi::WifiStaDevice>>> =
        static_cell::StaticCell::new();

    let stack = STACK_RAW.init_with(|| {
        embassy_net::Stack::new(
            wifi_interface,
            net_config,
            STACK_RESOURCES_RAW.init_with(|| embassy_net::StackResources::<3>::new()),
            1234,
        )
    });

    spawner.spawn(connection(controller)).unwrap();
    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(network_watchdog(stack, agent)).unwrap();

    stack
}

#[embassy_executor::task]
async fn network_watchdog(
    stack: &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static, esp_wifi::wifi::WifiStaDevice>>,
    mut agent: edgeless_embedded::agent::EmbeddedAgent,
) {
    loop {
        if stack.is_link_up() {
            break;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}. Registering with the Orchestrator.", config.address);
            agent.register(config.address.address()).await;
            log::info!("Registered with the Orchestrator.");
            break;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
async fn connection(mut controller: esp_wifi::wifi::WifiController<'static>) {
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            esp_wifi::wifi::WifiState::StaConnected => {
                controller.wait_for_event(esp_wifi::wifi::WifiEvent::StaDisconnected).await;
                embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = esp_wifi::wifi::Configuration::Client(esp_wifi::wifi::ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            log::info!("Starting wifi. SSID: {}", SSID);
            controller.start().await.unwrap();
        }

        log::info!("Attempt to connect.");
        match controller.connect().await {
            Ok(_) => log::info!("Wifi connected!"),
            Err(e) => {
                log::error!("Failed to connect to wifi: {e:?}");
                embassy_time::Timer::after(embassy_time::Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static, esp_wifi::wifi::WifiStaDevice>>) {
    stack.run().await
}
