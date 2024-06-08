//! ESP32-based firmware to power an openmoto `kontroller`.

#![allow(clippy::multiple_crate_versions)]

use esp32_nimble::BLEDevice;
use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    timer::EspTaskTimerService,
};

pub mod input;
pub mod key;
mod kontroller;
mod led;

use kontroller::Kontroller;
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
        led::Blinker::new(Led::new(peripherals.pins.gpio6)?, timer_svc.timer_async()?);

    let input_reporter = input::Reporter::new([
        (
            input::DirectionalPad::Up.into(),
            peripherals.pins.gpio0.downgrade(),
        ),
        (
            input::DirectionalPad::Left.into(),
            peripherals.pins.gpio1.downgrade(),
        ),
        (
            input::DirectionalPad::Right.into(),
            peripherals.pins.gpio2.downgrade(),
        ),
        (
            input::DirectionalPad::Down.into(),
            peripherals.pins.gpio3.downgrade(),
        ),
        (input::Key::Enter, peripherals.pins.gpio4.downgrade()),
        (input::Key::Function, peripherals.pins.gpio5.downgrade()),
    ])?;

    let kontroller = Kontroller::new(
        timer_svc,
        kontroller::Subsystems {
            ble_device,
            led_blinker,
            input_reporter,
        },
    );

    log::debug!("Peripherals fully initialized");

    task::block_on(kontroller.start())?;

    Ok(())
}
