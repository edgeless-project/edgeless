// Based on https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs
use embedded_svc::wifi::Wifi;

use hal::prelude::*;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

// https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs
macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: static_cell::StaticCell<T> = static_cell::StaticCell::new();
        let (x,) = STATIC_CELL.init(($val,));
        x
    }};
}

pub async fn init(
    spawner: embassy_executor::Spawner,
    timer: esp_wifi::EspWifiTimer,
    rng: hal::Rng,
    radio_clock_control: hal::system::RadioClockControl,
    clocks: hal::clock::Clocks<'static>,
    radio: hal::peripherals::RADIO,
) -> &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static>> {
    let init = esp_wifi::initialize(esp_wifi::EspWifiInitFor::Wifi, timer, rng.clone(), radio_clock_control, &clocks).unwrap();

    let (wifi, _) = radio.split();

    let (wifi_interface, controller) = esp_wifi::wifi::new_with_mode(&init, wifi, esp_wifi::wifi::WifiMode::Sta).unwrap();

    let net_config = embassy_net::Config::dhcpv4(Default::default());

    let stack = static_cell::make_static!(embassy_net::Stack::new(
        wifi_interface,
        net_config,
        singleton!(embassy_net::StackResources::<3>::new()),
        1234
    ));

    spawner.spawn(connection(controller)).unwrap();
    spawner.spawn(net_task(stack)).unwrap();
    spawner.spawn(network_watchdog(stack)).unwrap();

    stack
}

#[embassy_executor::task]
async fn network_watchdog(stack: &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static>>) {
    loop {
        if stack.is_link_up() {
            break;
        }
        embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}", config.address);
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
            let client_config = embedded_svc::wifi::Configuration::Client(embedded_svc::wifi::ClientConfiguration {
                ssid: SSID.into(),
                password: PASSWORD.into(),
                auth_method: embedded_svc::wifi::AuthMethod::WPA2Personal,
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
async fn net_task(stack: &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static>>) {
    stack.run().await
}
