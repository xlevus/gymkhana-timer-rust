#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use defmt::{error, info, panic as _panic};
use display_interface::DisplayError;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::{self, peripherals};
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config, I2c},
    timer::timg::TimerGroup,
};
use esp_storage::FlashStorage;
use ssd1306::{
    I2CDisplayInterface, Ssd1306Async, mode::DisplayConfigAsync, mode::TerminalModeError,
    prelude::DisplayRotation, prelude::DisplaySize128x32,
};
use alloc::string::String;
use wifi_caddy::ConfigHandle;
use wifi_caddy_proc::WifiCaddyConfig;
use esp_wifi_caddy::config_storage::ConfigError;

#[panic_handler]
fn panic(pinfo: &core::panic::PanicInfo) -> ! {
    error!("Panic!");
    loop {}
}

extern crate alloc;


/// App config: WiFi credentials and example string and integer.
#[derive(Clone, Debug, Default, WifiCaddyConfig)]
#[config_server]
#[config_notify]
#[config_ui(
    page_heading = "My App",
    title = "My App - Configuration",
)]
pub struct AppConfig {
    #[config_store(env_default = "WIFI_SSID", notify = "Wifi")]
    #[config_form(page = "Network", fieldset = "WiFi", help = "Network name (SSID)")]
    wifi_ssid: String,
    #[config_store(env_default = "WIFI_PASS", notify = "Wifi")]
    #[config_form(
        page = "Network",
        fieldset = "WiFi",
        input_type = "password",
        help = "WiFi password"
    )]
    wifi_pass: String,
    #[config_store(notify = "Example")]
    #[config_form(page = "Example", fieldset = "Example", help = "String field")]
    example_string: String,
    #[config_store(notify = "Example")]
    #[config_form(page = "Example", fieldset = "Example", help = "Integer field")]
    example_integer: u32,
}

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

    error!("ERROR");

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let p = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(p.TIMG0);
    let sw_interrupt = esp_hal::interrupt::software::SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");

    // TODO: Spawn some tasks
    let _ = spawner;
    let led = Output::new(p.GPIO21, Level::High, OutputConfig::default());

    // I2c setup
    let i2c = match I2c::new(p.I2C0, Config::default()) {
        Ok(res) => res.with_scl(p.GPIO9).with_sda(p.GPIO8).into_async(),
        Err(_) => {
            _panic!("Unable to set up i2c");
        }
    };

    // Display setup
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306Async::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
        .into_terminal_mode();

    match display.init().await {
        Ok(_) => {
            info!("Display initialised");
            let _ = display.clear().await;
            let _ = display.write_str("Display Ok!\n").await;
        },
        Err(v) => match v {
            TerminalModeError::InterfaceError(DisplayError::InvalidFormatError) => {
                error!("Interface Error(InvalidFormat)")
            }
            TerminalModeError::InterfaceError(DisplayError::BusWriteError) => {
                error!("Interface Error(BusWriteError)")
            }
            TerminalModeError::InterfaceError(_) => {
                error!("Interface Error(Other)")
            }
            TerminalModeError::OutOfBounds => error!("OutOfBounds"),
            TerminalModeError::Uninitialized => error!("Unitialized"),
        },
    }

    // Wifi
    let flash = FlashStorage::new(p.FLASH);
    // info!("Wifi1");
    // let (wifi_stacks, wifi_sender, config, config_rx) =
    //     esp_wifi_caddy::wifi_init!(AppConfig, spawner, p.WIFI, flash, "config")
    //         .expect("wifi_init");
    match esp_wifi_caddy::wifi_init!(AppConfig, spawner, p.WIFI, flash, "config") {
        Ok((wifi_stacks, wifi_sender, config, config_rx)) => {
            info!("Wifi Initialized");
            wifi_sender.send(esp_wifi_caddy::WifiCaddyCommand::APUp(String::from("xyz-"))).await;
            info!("Ap Up!");
        },
        Err(v) => {
            display.write_str("Wifi Error.\n").await.expect("Err");
            error!("WifiError: {}", v);
            panic!("");
        }
    }
    
    // info!("Wifi2");
    // wifi_sender.send(esp_wifi_caddy::WifiCaddyCommand::APUp(String::from("wifi-example-"))).await;
    // info!("Wifi3");

    spawner.spawn(blink(led.into())).ok();

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v~1.0/examples
}
