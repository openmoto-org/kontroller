//! ESP32-based firmware to power an openmoto `kontroller`.

#![allow(clippy::multiple_crate_versions)]

use std::time::{Duration, Instant};

use ble::Config;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use esp32_nimble::BLEDevice;
use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    timer::{EspAsyncTimer, EspTaskTimerService},
};

mod ble;
mod hid;
pub mod input;
pub mod key;
mod keycode;
// mod kontroller;
mod led;

use futures::channel::mpsc::channel;
use led::Led;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::debug!("Initializing peripherals...");

    let peripherals = Peripherals::take()?;
    let timer_svc = EspTaskTimerService::new()?;
    let ble_device = BLEDevice::take();

    let led_blinker =
        led::Blinker::new(Led::new(peripherals.pins.gpio7)?, timer_svc.timer_async()?);

    let mut input_reporter = input::Reporter::new([
        (input::Key::Enter, peripherals.pins.gpio8.downgrade()),
        (input::Key::Up, peripherals.pins.gpio9.downgrade()),
        (input::Key::Right, peripherals.pins.gpio10.downgrade()),
        (input::Key::Left, peripherals.pins.gpio11.downgrade()),
        (input::Key::Down, peripherals.pins.gpio12.downgrade()),
        (input::Key::Fn1, peripherals.pins.gpio4.downgrade()),
        (input::Key::Fn2, peripherals.pins.gpio5.downgrade()),
        (input::Key::Fn3, peripherals.pins.gpio6.downgrade()),
    ])?;

    let mut ble_server = ble::Server::new(&ble::Config {
        device_name: "DMD CTL 8K",
    })?;

    let (report_tx, report_rx) = channel::<hid::Report>(8);

    let mut led_blinker_timer = timer_svc.timer_async()?;
    let mut ble_server_timer = timer_svc.timer_async()?;
    let mut input_reporter_timer = timer_svc.timer_async()?;

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        futures::try_join!(
            handle_led_status(led_blinker_timer, &led_blinker),
            input_reporter.start(Instant::now, &mut input_reporter_timer, report_tx),
            ble_server.start(&mut ble_server_timer, report_rx),
        )
    })?;

    Ok(())
}

async fn handle_led_status<'d>(
    mut timer: EspAsyncTimer,
    led_blinker: &led::Blinker<'d>,
) -> anyhow::Result<()> {
    let timer: &mut EspAsyncTimer = timer.every(Duration::from_millis(500))?;

    loop {
        timer.tick().await?;
        led_blinker.long_blink().await?;
    }
}
