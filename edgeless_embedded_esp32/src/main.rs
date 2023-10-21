// Based on https://github.com/esp-rs/esp-hal/blob/main/esp32-hal/examples/embassy_hello_world.rs, https://github.com/esp-rs/esp-template/blob/main/src/main.rs & https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]

extern crate alloc;

pub mod agent;
pub mod coap;
pub mod dataplane;
pub mod epaper_display;
pub mod invocation;
pub mod mock_display;
pub mod mock_sensor;
pub mod resource;
pub mod resource_configuration;
pub mod scd30_sensor;
pub mod wifi;

use epd_waveshare::prelude::WaveshareDisplay;
use esp_backtrace as _;
use hal::prelude::*;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

static RNG: once_cell::sync::OnceCell<hal::Rng> = once_cell::sync::OnceCell::new();

const ESP_GETRANDOM_ERROR: u32 = getrandom::Error::CUSTOM_START + 1;

const NODE_ID: uuid::Uuid = uuid::uuid!("0827240a-3050-4604-bf3e-564c41c77106");

const COAP_PEERS: [(uuid::Uuid, smoltcp::wire::IpEndpoint); 1] = [(
    uuid::uuid!("fda6ce79-46df-4f96-a0d2-456f720f606c"),
    smoltcp::wire::IpEndpoint {
        addr: embassy_net::IpAddress::v4(192, 168, 2, 61),
        port: 7002,
    },
)];

use crate::epaper_display::EPaper;

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

getrandom::register_custom_getrandom!(esp_getrandom);

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    esp_println::println!("Start Edgeless Embedded.");

    // https://github.com/esp-rs/esp-template/blob/main/src/main.rs
    init_heap();

    let peripherals = hal::peripherals::Peripherals::take();
    let io = hal::IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let system = peripherals.SYSTEM.split();

    let clocks = hal::clock::ClockControl::max(system.clock_control).freeze();
    let timer_group0 = hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks);
    let timer_group1 = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks);

    let rng = hal::Rng::new(peripherals.RNG);
    assert!(RNG.set(rng.clone()).is_ok());

    hal::embassy::init(&clocks, timer_group0.timer0);

    let spi = static_cell::make_static!(hal::spi::SpiBusController::from_spi(hal::spi::Spi::new_no_cs_no_miso(
        peripherals.SPI2,
        io.pins.gpio18,
        io.pins.gpio23,
        100u32.kHz(),
        hal::spi::SpiMode::Mode0,
        &clocks
    )));

    let i2c = hal::i2c::I2C::new(peripherals.I2C0, io.pins.gpio33, io.pins.gpio32, 50u32.kHz(), &clocks);

    let mut i2c_delay = hal::delay::Delay::new(&clocks);
    i2c_delay.delay_ms(2000u32);

    let scd = sensor_scd30::Scd30::new(i2c, i2c_delay).unwrap();

    let scd30 = static_cell::make_static!(scd30_sensor::SCD30SensorWrapper { sensor: scd });

    let mut spi_dev = spi.add_device(io.pins.gpio5);
    let busy_pin = io.pins.gpio4.into_floating_input();
    let dc_pin = io.pins.gpio17.into_push_pull_output();
    let rst_pin = io.pins.gpio16.into_push_pull_output();
    let mut epaper_delay = hal::delay::Delay::new(&clocks);

    let executor = static_cell::make_static!(hal::embassy::executor::Executor::new());

    let epd = epd_waveshare::epd2in13_lillygo::Epd2in13::new(&mut spi_dev, busy_pin, dc_pin, rst_pin, &mut epaper_delay, None).unwrap();

    let display = epd_waveshare::epd2in13_lillygo::Display2in13::default();

    let display = static_cell::make_static!(epaper_display::LillyGoEPaper {
        spi_dev: spi_dev,
        epd: epd,
        display: display,
        delay: epaper_delay
    });

    display.set_text("Edgeless");

    executor.run(|spawner| {
        spawner.spawn(edgeless(
            spawner,
            timer_group1.timer0,
            rng,
            system.radio_clock_control,
            clocks,
            peripherals.RADIO,
            display,
            scd30,
        ));
    });

    loop {}
}

#[embassy_executor::task]
async fn edgeless(
    spawner: embassy_executor::Spawner,
    timer: esp_wifi::EspWifiTimer,
    rng: hal::Rng,
    radio_clock_control: hal::system::RadioClockControl,
    clocks: hal::clock::Clocks<'static>,
    radio: hal::peripherals::RADIO,
    display: &'static mut dyn epaper_display::EPaper,
    scd30: &'static mut dyn scd30_sensor::Sensor,
) {
    log::info!("Edgeless Embedded Async Main");

    let stack = wifi::init(spawner.clone(), timer, rng, radio_clock_control, clocks, radio).await;

    let mock_sensor = mock_sensor::MockSensor::new().await;
    let scd30_sensor = scd30_sensor::SCD30Sensor::new(scd30).await;
    let mock_display = mock_display::MockDisplay::new().await;
    let epaper_display = epaper_display::EPaperDisplay::new(display).await;

    let resources = static_cell::make_static!([mock_display, epaper_display, mock_sensor, scd30_sensor]);

    let resource_registry = agent::EmbeddedAgent::new(spawner, NODE_ID.clone(), resources).await;

    spawner.spawn(coap::coap_task(
        stack,
        resource_registry.upstream_receiver().unwrap(),
        resource_registry.clone(),
    ));
}
