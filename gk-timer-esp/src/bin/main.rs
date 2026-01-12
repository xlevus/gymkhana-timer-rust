#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{
    clock::CpuClock, efuse::KEY_PURPOSE_0, gpio::{Level, Output, OutputConfig}, i2c::master::{Config, I2c}, timer::timg::TimerGroup
};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::DisplayRotation, prelude::DisplaySize128x32};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();


#[embassy_executor::task]
async fn blink(mut pin: Output<'static>) {
    loop {
        pin.toggle();
        Timer::after_millis(500).await;
    }
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.1.0

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(p.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);


    info!("Embassy initialized!");

    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(&radio_init, p.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    // TODO: Spawn some tasks
    let _ = spawner;
    let led = Output::new(p.GPIO21, Level::High, OutputConfig::default());

    // let i2c = I2c::new(p.I2C0, Config::default()).unwrap()
    //     .with_scl(p.GPIO8)
    //     .with_sda(p.GPIO9)
    //     .into_async();

    // let interface = I2CDisplayInterface::new(i2c);
    // let mut display= Ssd1306Async::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
    //     .into_terminal_mode();

    // display.init().await.unwrap();

    spawner.spawn(blink(led.into())).ok();

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
