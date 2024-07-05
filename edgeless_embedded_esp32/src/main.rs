// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
// Based on https://github.com/esp-rs/esp-hal/blob/main/esp32-hal/examples/embassy_hello_world.rs, https://github.com/esp-rs/esp-template/blob/main/src/main.rs & https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

#![no_std]
#![no_main]

extern crate alloc;

#[cfg(feature = "epaper_2_13")]
pub mod epaper_display_impl;
#[cfg(feature = "scd30")]
pub mod scd30_sensor_impl;
pub mod wifi;

use edgeless_embedded::agent::EmbeddedAgent;
#[cfg(feature = "epaper_2_13")]
use edgeless_embedded::resource::epaper_display::EPaper;
use embedded_hal::delay::DelayNs;
use epd_waveshare::prelude::*;
use esp_backtrace as _;
use hal::prelude::*;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

static RNG: once_cell::sync::OnceCell<hal::rng::Rng> = once_cell::sync::OnceCell::new();

const ESP_GETRANDOM_ERROR: u32 = getrandom::Error::CUSTOM_START + 1;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

static mut APP_CORE_STACK: hal::cpu_control::Stack<8192> = hal::cpu_control::Stack::new();

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
    esp_println::logger::init_logger(log::LevelFilter::Info);
    esp_println::println!("Start Edgeless Embedded.");

    // https://github.com/esp-rs/esp-template/blob/main/src/main.rs
    init_heap();

    let peripherals = hal::peripherals::Peripherals::take();
    #[allow(unused_variables)]
    let io = hal::gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let system = hal::system::SystemControl::new(peripherals.SYSTEM);

    let clocks = hal::clock::ClockControl::max(system.clock_control).freeze();
    let timer_group0 = hal::timer::timg::TimerGroup::new_async(peripherals.TIMG0, &clocks);
    let timer_group1 = hal::timer::timg::TimerGroup::new(peripherals.TIMG1, &clocks, None);

    let rng = hal::rng::Rng::new(peripherals.RNG);
    assert!(RNG.set(rng.clone()).is_ok());

    esp_hal_embassy::init(&clocks, timer_group0);

    let mut cpu_control = hal::cpu_control::CpuControl::new(peripherals.CPU_CTRL);

    #[cfg(feature = "epaper_2_13")]
    let display_wrapper = {
        let spi = hal::spi::master::Spi::new(peripherals.SPI2, 100u32.kHz(), hal::spi::SpiMode::Mode0, &clocks)
            .with_sck(io.pins.gpio18)
            .with_mosi(io.pins.gpio23);

        let display_pin = hal::gpio::Output::new(io.pins.gpio5, hal::gpio::Level::Low);

        let mut spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, display_pin).unwrap();
        let busy_pin = hal::gpio::Input::new(io.pins.gpio4, hal::gpio::Pull::None);
        let dc_pin = hal::gpio::Output::new(io.pins.gpio17, hal::gpio::Level::High);
        let rst_pin = hal::gpio::Output::new(io.pins.gpio16, hal::gpio::Level::High);
        let mut epaper_delay = hal::delay::Delay::new(&clocks);

        let epd = epd_waveshare::epd2in13_lillygo::Epd2in13::new(&mut spi_dev, busy_pin, dc_pin, rst_pin, &mut epaper_delay, None).unwrap();

        let display = epd_waveshare::epd2in13_lillygo::Display2in13::default();

        static DISPLAY_WRAPPER_RAW: static_cell::StaticCell<
            epaper_display_impl::LillyGoEPaper<
                embedded_hal_bus::spi::ExclusiveDevice<
                    hal::spi::master::Spi<'_, hal::peripherals::SPI2, hal::spi::FullDuplexMode>,
                    hal::gpio::Output<hal::gpio::Gpio5>,
                    embedded_hal_bus::spi::NoDelay,
                >,
                hal::gpio::Input<hal::gpio::Gpio4>,
                hal::gpio::Output<hal::gpio::Gpio17>,
                hal::gpio::Output<hal::gpio::Gpio16>,
                hal::delay::Delay,
            >,
        > = static_cell::StaticCell::new();
        let display_wrapper = DISPLAY_WRAPPER_RAW.init_with(|| epaper_display_impl::LillyGoEPaper {
            spi_dev: spi_dev,
            epd: epd,
            display: display,
            delay: epaper_delay,
        });

        display_wrapper
    };

    #[cfg(feature = "scd30")]
    let sensor_wrapper = {
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

        static SENSOR_WRAPPER_RAW: static_cell::StaticCell<
            scd30_sensor_impl::SCD30SensorWrapper<hal::i2c::I2C<'_, hal::peripherals::I2C0, hal::Blocking>, hal::delay::Delay, hal::i2c::Error>,
        > = static_cell::StaticCell::new();

        let sensor_wrapper = SENSOR_WRAPPER_RAW.init_with(|| scd30_sensor_impl::SCD30SensorWrapper { sensor: scd });
        sensor_wrapper
    };

    static CHANNEL_RAW: static_cell::StaticCell<
        embassy_sync::channel::Channel<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            edgeless_embedded::resource::scd30_sensor::Measurement,
            2,
        >,
    > = static_cell::StaticCell::new();
    let channel = CHANNEL_RAW.init_with(|| {
        embassy_sync::channel::Channel::<
            embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
            edgeless_embedded::resource::scd30_sensor::Measurement,
            2,
        >::new()
    });

    let sender = channel.sender();
    let receiver = channel.receiver();

    static DISPLAY_CHANNEL_RAW: static_cell::StaticCell<
        embassy_sync::channel::Channel<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
    > = static_cell::StaticCell::new();
    let display_channel = DISPLAY_CHANNEL_RAW
        .init_with(|| embassy_sync::channel::Channel::<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>::new());

    let display_sender = display_channel.sender();
    let display_receiver = display_channel.receiver();

    let _other_core = cpu_control
        .start_app_core(unsafe { &mut *core::ptr::addr_of_mut!(APP_CORE_STACK) }, move || {
            static IO_EXECUTOR_RAW: static_cell::StaticCell<esp_hal_embassy::Executor> = static_cell::StaticCell::new();
            let io_executor = IO_EXECUTOR_RAW.init_with(|| esp_hal_embassy::Executor::new());

            io_executor.run(|spawner| {
                #[cfg(feature = "epaper_2_13")]
                display_wrapper.set_text("Edgeless");
                #[cfg(feature = "scd30")]
                spawner.spawn(io_task(spawner, sender, sensor_wrapper));
                #[cfg(feature = "epaper_2_13")]
                spawner.spawn(edgeless_embedded::resource::epaper_display::display_writer(
                    display_receiver,
                    display_wrapper,
                ));
            });
        })
        .unwrap();

    static EXECUTOR_RAW: static_cell::StaticCell<esp_hal_embassy::Executor> = static_cell::StaticCell::new();
    let executor = EXECUTOR_RAW.init_with(|| esp_hal_embassy::Executor::new());

    executor.run(|spawner| {
        spawner.spawn(edgeless(
            spawner,
            timer_group1.timer0,
            rng,
            peripherals.RADIO_CLK,
            clocks,
            peripherals.WIFI,
            receiver,
            display_sender,
        ));
    });
}

#[embassy_executor::task]
async fn registration(agent: EmbeddedAgent) {
    let mut agent = agent;
    embassy_time::Timer::after_millis(30000).await;
    log::info!("Registration with the Orchestrator!");
    agent.register().await;
    log::info!("Registration done!");
}

#[embassy_executor::task]
async fn io_task(
    spawner: embassy_executor::Spawner,
    sender: embassy_sync::channel::Sender<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        edgeless_embedded::resource::scd30_sensor::Measurement,
        2,
    >,
    sensor_wrapper: &'static mut dyn edgeless_embedded::resource::scd30_sensor::Sensor,
) {
    spawner
        .spawn(edgeless_embedded::resource::scd30_sensor::scd30_reader_task(sensor_wrapper, sender))
        .unwrap();
}

#[embassy_executor::task]
async fn edgeless(
    spawner: embassy_executor::Spawner,
    timer: hal::timer::timg::Timer<hal::timer::timg::TimerX<hal::peripherals::TIMG1>, hal::Blocking>,
    rng: hal::rng::Rng,
    radio_clock_control: hal::peripherals::RADIO_CLK,
    clocks: hal::clock::Clocks<'static>,
    wifi: hal::peripherals::WIFI,
    sensor_scd_receiver: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
        edgeless_embedded::resource::scd30_sensor::Measurement,
        2,
    >,
    display_sender: embassy_sync::channel::Sender<'static, embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::String<1500>, 2>,
) {
    log::info!("Edgeless Embedded Async Main");

    static RX_BUF_RAW: static_cell::StaticCell<[u8; 5000]> = static_cell::StaticCell::new();
    let rx_buf = RX_BUF_RAW.init_with(|| [0 as u8; 5000]);
    static RX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
    let rx_meta = RX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);
    static TX_BUF_RAW: static_cell::StaticCell<[u8; 5000]> = static_cell::StaticCell::new();
    let tx_buf = TX_BUF_RAW.init_with(|| [0 as u8; 5000]);
    static TX_META_RAW: static_cell::StaticCell<[embassy_net::udp::PacketMetadata; 10]> = static_cell::StaticCell::new();
    let tx_meta = TX_META_RAW.init_with(|| [embassy_net::udp::PacketMetadata::EMPTY; 10]);

    let stack = wifi::init(spawner.clone(), timer, rng, radio_clock_control, clocks, wifi).await;
    let sock = embassy_net::udp::UdpSocket::new(stack, rx_meta, rx_buf, tx_meta, tx_buf);

    let display_resource = edgeless_embedded::resource::epaper_display::EPaperDisplay::new(display_sender).await;

    let sensor_scd30_resource = edgeless_embedded::resource::scd30_sensor::SCD30Sensor::new(sensor_scd_receiver).await;

    static RESOURCES_RAW: static_cell::StaticCell<[&'static mut dyn edgeless_embedded::resource::ResourceDyn; 2]> = static_cell::StaticCell::new();
    let resources = RESOURCES_RAW.init_with(|| [sensor_scd30_resource, display_resource]);

    let resource_registry = edgeless_embedded::agent::EmbeddedAgent::new(spawner, NODE_ID.clone(), resources, "coap://192.168.2.60").await;

    spawner.spawn(edgeless_embedded::coap::coap_task(
        sock,
        resource_registry.upstream_receiver().unwrap(),
        resource_registry.clone(),
    ));

    spawner.spawn(registration(resource_registry.clone()));
}
