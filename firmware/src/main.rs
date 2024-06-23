//! ESP32-based firmware to power an openmoto `kontroller`.

#![allow(clippy::multiple_crate_versions)]

use embassy_time::{Duration, Instant, Timer};
use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    timer::{EspAsyncTimer, EspTaskTimerService},
};

mod ble;
mod hid;
pub mod key;
mod kontroller;
mod led;
#[allow(clippy::pedantic)]
mod proto;

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

    let led_blinker = led::Blinker::from(Led::new(peripherals.pins.gpio7)?);

    let mut input_reporter = kontroller::Reporter::new([
        (kontroller::Key::Enter, peripherals.pins.gpio8.downgrade()),
        (kontroller::Key::Up, peripherals.pins.gpio9.downgrade()),
        (kontroller::Key::Right, peripherals.pins.gpio10.downgrade()),
        (kontroller::Key::Left, peripherals.pins.gpio11.downgrade()),
        (kontroller::Key::Down, peripherals.pins.gpio12.downgrade()),
        (kontroller::Key::Fn1, peripherals.pins.gpio4.downgrade()),
        (kontroller::Key::Fn2, peripherals.pins.gpio5.downgrade()),
        (kontroller::Key::Fn3, peripherals.pins.gpio6.downgrade()),
    ])?;

    let mut ble_server = ble::Server::new(&ble::Config {
        device_name: "DMD CTL 8K",
    })?;

    let (report_tx, report_rx) = channel::<hid::Report>(8);

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        futures::try_join!(
            handle_led_status(led_blinker),
            input_reporter.start(Instant::now, report_tx),
            ble_server.start(report_rx),
        )
    })?;

    Ok(())
}

async fn handle_led_status(mut led_blinker: led::Blinker<'_>) -> anyhow::Result<()> {
    loop {
        Timer::after(Duration::from_millis(500)).await;
        led_blinker.long_blink().await?;
    }
}
