//! ESP32-based firmware to power an openmoto `kontroller`.

#![allow(clippy::multiple_crate_versions)]

use embassy_time::Instant;
use esp_idf_svc::hal::{gpio::IOPin, peripherals::Peripherals, task};

mod ble;
mod hid;
pub mod key;
mod kontroller;
mod led;
#[allow(clippy::pedantic)]
mod proto;

use futures::channel::mpsc::channel;
use led::Led;
use proto::kontroller::{hid::v1::KeyCode, v1::Button};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::debug!("Initializing peripherals...");

    let peripherals = Peripherals::take()?;

    let mut led_blinker = led::Blinker::from(Led::new(peripherals.pins.gpio7)?);

    let mut kontroller = kontroller::Kontroller::new(
        [
            (Button::Enter, peripherals.pins.gpio8.downgrade()),
            (Button::Up, peripherals.pins.gpio9.downgrade()),
            (Button::Right, peripherals.pins.gpio10.downgrade()),
            (Button::Left, peripherals.pins.gpio11.downgrade()),
            (Button::Down, peripherals.pins.gpio12.downgrade()),
            (Button::Fn1, peripherals.pins.gpio4.downgrade()),
            (Button::Fn2, peripherals.pins.gpio5.downgrade()),
            (Button::Fn3, peripherals.pins.gpio6.downgrade()),
        ],
        proto::kontroller::v1::Konfiguration {
            buttons_poll_interval_micros: 500,
            keymap: Some(kontroller::make_keymap([
                (Button::Enter, KeyCode::Enter),
                (Button::Up, KeyCode::Up),
                (Button::Right, KeyCode::Right),
                (Button::Left, KeyCode::Left),
                (Button::Down, KeyCode::Down),
                (Button::Fn1, KeyCode::F7),
                (Button::Fn2, KeyCode::F6),
                (Button::Fn3, KeyCode::F5),
            ])),
        },
    )?;

    let mut ble_server = ble::Server::initialize(&ble::Config {
        device_name: "DMD CTL 8K",
    })?;

    let (report_tx, report_rx) = channel::<hid::Report>(1);

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        futures::try_join!(
            kontroller.start(Instant::now, report_tx),
            ble_server.start(report_rx, &mut led_blinker),
        )
    })?;

    Ok(())
}
