// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-hal/blob/main/esp32-hal/examples/embassy_hello_world.rs, https://github.com/esp-rs/esp-template/blob/main/src/main.rs & https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

pub mod wifi;

#[cfg(feature = "epaper_2_13")]
pub mod epaper_display_impl;
#[cfg(feature = "scd30")]
pub mod scd30_sensor_impl;

use edgeless_embedded::agent::EmbeddedAgent;
use esp_backtrace as _;
use hal::prelude::*;

#[cfg(feature = "epaper_2_13")]
use edgeless_embedded::resource::epaper_display::EPaper;
#[cfg(feature = "epaper_2_13")]
use epd_waveshare::prelude::*;

use embedded_hal::delay::DelayNs;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

static RNG: once_cell::sync::OnceCell<hal::rng::Rng> = once_cell::sync::OnceCell::new();

const ESP_GETRANDOM_ERROR: u32 = getrandom::Error::CUSTOM_START + 1;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: core::mem::MaybeUninit<[u8; HEAP_SIZE]> = core::mem::MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

fn esp_getrandom(dest: &mut [u8]) -> Result<(), getrandom::Error> {
    match RNG.get() {
        Some(rng) => {
            let mut rng = rng.clone();
            for dest_byte in dest {
                *dest_byte = rng.random() as u8;
            }
            Ok(())
        }
        None => Err(getrandom::Error::from(core::num::NonZeroU32::new(ESP_GETRANDOM_ERROR).unwrap())),
    }
}

#[allow(non_upper_case_globals)]
getrandom::register_custom_getrandom!(esp_getrandom);

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    esp_println::println!("Start Edgeless Embedded.");

    // https://github.com/esp-rs/esp-template/blob/main/src/main.rs
    init_heap();

    let peripherals = hal::peripherals::Peripherals::take();
    #[allow(unused_variables)]
    let io = hal::gpio::IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let system = peripherals.SYSTEM.split();

    let clocks = hal::clock::ClockControl::max(system.clock_control).freeze();
    let timer_group0 = hal::timer::TimerGroup::new_async(peripherals.TIMG0, &clocks);
    let timer_group1 = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks, None);

    let rng = hal::rng::Rng::new(peripherals.RNG);
    assert!(RNG.set(rng.clone()).is_ok());

    hal::embassy::init(&clocks, timer_group0);

    #[cfg(feature = "epaper_2_13")]
    let display: Option<&'static mut dyn edgeless_embedded::resource::epaper_display::EPaper> = {
        let spi = static_cell::make_static!(
            hal::spi::master::Spi::new(peripherals.SPI2, 100u32.kHz(), hal::spi::SpiMode::Mode0, &clocks)
                .with_sck(io.pins.gpio18)
                .with_mosi(io.pins.gpio23)
        );

        let mut spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, io.pins.gpio5.into_push_pull_output()).unwrap();
        let busy_pin = io.pins.gpio4.into_floating_input();
        let dc_pin = io.pins.gpio17.into_push_pull_output();
        let rst_pin = io.pins.gpio16.into_push_pull_output();
        let mut epaper_delay = hal::delay::Delay::new(&clocks);

        let epd = epd_waveshare::epd2in13_lillygo::Epd2in13::new(&mut spi_dev, busy_pin, dc_pin, rst_pin, &mut epaper_delay, None).unwrap();

        let display = epd_waveshare::epd2in13_lillygo::Display2in13::default();

        let display_wrapper = static_cell::make_static!(epaper_display_impl::LillyGoEPaper {
            spi_dev: spi_dev,
            epd: epd,
            display: display,
            delay: epaper_delay
        });

        display_wrapper.set_text("Edgeless");

        Some(display_wrapper)
    };
    #[cfg(not(feature = "epaper_2_13"))]
    let display: Option<&'static mut dyn edgeless_embedded::resource::epaper_display::EPaper> = None;

    #[cfg(feature = "scd30")]
    let scd30: Option<&'static mut dyn edgeless_embedded::resource::scd30_sensor::Sensor> = {
        let i2c = hal::i2c::I2C::new_with_timeout(
            peripherals.I2C0,
            io.pins.gpio33,
            io.pins.gpio32,
            50u32.kHz(),
            &clocks,
            Some(0xFFFFF),
            None,
        );

        let mut i2c_delay = hal::delay::Delay::new(&clocks);
        i2c_delay.delay_ms(5000u32);

        let scd = sensor_scd30::Scd30::new(i2c, i2c_delay).unwrap();

        Some(static_cell::make_static!(scd30_sensor_impl::SCD30SensorWrapper { sensor: scd }))
    };
    #[cfg(not(feature = "scd30"))]
    let scd30: Option<&'static mut dyn edgeless_embedded::resource::scd30_sensor::Sensor> = None;

    let executor = static_cell::make_static!(hal::embassy::executor::Executor::new());

    executor.run(|spawner| {
        spawner.spawn(edgeless(
            spawner,
            timer_group1.timer0,
            rng,
            system.radio_clock_control,
            clocks,
            peripherals.WIFI,
            display,
            scd30,
        ));
    });

    #[allow(unreachable_code)]
    loop {}
}

#[embassy_executor::task]
async fn registration(agent: EmbeddedAgent) {
    let mut agent = agent;
    let mut delay = embassy_time::Delay {};
    loop {
        delay.delay_ms(2000);
        agent.register().await;
    }
}

#[embassy_executor::task]
async fn edgeless(
    spawner: embassy_executor::Spawner,
    timer: hal::timer::Timer<hal::timer::TimerX<hal::peripherals::TIMG1>, hal::Blocking>,
    rng: hal::rng::Rng,
    radio_clock_control: hal::system::RadioClockControl,
    clocks: hal::clock::Clocks<'static>,
    wifi: hal::peripherals::WIFI,
    display: Option<&'static mut dyn edgeless_embedded::resource::epaper_display::EPaper>,
    scd30: Option<&'static mut dyn edgeless_embedded::resource::scd30_sensor::Sensor>,
) {
    log::info!("Edgeless Embedded Async Main");

    let rx_buf = static_cell::make_static!([0 as u8; 5000]);
    let rx_meta = static_cell::make_static!([embassy_net::udp::PacketMetadata::EMPTY; 10]);
    let tx_buf = static_cell::make_static!([0 as u8; 5000]);
    let tx_meta = static_cell::make_static!([embassy_net::udp::PacketMetadata::EMPTY; 10]);

    let stack = wifi::init(spawner.clone(), timer, rng, radio_clock_control, clocks, wifi).await;
    let sock = embassy_net::udp::UdpSocket::new(stack, rx_meta, rx_buf, tx_meta, tx_buf);

    let sensor_scd30 = if let Some(scd30) = scd30 {
        edgeless_embedded::resource::scd30_sensor::SCD30Sensor::new(scd30).await
    } else {
        edgeless_embedded::resource::mock_sensor::MockSensor::new().await
    };

    let display = if let Some(display) = display {
        edgeless_embedded::resource::epaper_display::EPaperDisplay::new(display).await
    } else {
        edgeless_embedded::resource::mock_display::MockDisplay::new().await
    };

    let resources = static_cell::make_static!([display, sensor_scd30]);

    let resource_registry = edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID.clone(), resources, "coap://192.168.2.60").await;

    spawner.spawn(edgeless_embedded::coap::coap_task(
        sock,
        resource_registry.upstream_receiver().unwrap(),
        resource_registry.clone(),
    ));

    spawner.spawn(registration(resource_registry.clone()));
}
