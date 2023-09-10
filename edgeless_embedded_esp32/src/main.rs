// Based on https://github.com/esp-rs/esp-hal/blob/main/esp32-hal/examples/embassy_hello_world.rs, https://github.com/esp-rs/esp-template/blob/main/src/main.rs & https://github.com/esp-rs/esp-wifi/blob/main/examples-esp32/examples/embassy_dhcp.rs

#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

pub mod wifi;

use esp_backtrace as _;

use hal::{
    prelude::*,
};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: core::mem::MaybeUninit<[u8; HEAP_SIZE]> = core::mem::MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

#[entry]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();
    log::info!("Start Edgeless Embedded.");
    
    // https://github.com/esp-rs/esp-template/blob/main/src/main.rs
    init_heap();
    
    let peripherals = hal::peripherals::Peripherals::take();
    let mut system = peripherals.DPORT.split();
    
    let clocks =  hal::clock::ClockControl::max(system.clock_control).freeze();
    let timer_group0 = hal::timer::TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let timer_group1 = hal::timer::TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );

    let rng = hal::Rng::new(peripherals.RNG);

    hal::embassy::init(&clocks, timer_group0.timer0);

    let executor = static_cell::make_static!(hal::embassy::executor::Executor::new());

    executor.run(|spawner| {
        spawner.spawn(wifi::init(spawner.clone(), timer_group1.timer0, rng, system.radio_clock_control, clocks, peripherals.RADIO)).unwrap();
    });

}