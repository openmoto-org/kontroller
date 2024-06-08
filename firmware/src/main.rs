//! ESP32-based firmware to power an openmoto `kontroller`.

#![allow(clippy::multiple_crate_versions)]

use std::time::Duration;

use esp32_nimble::{BLEAdvertisementData, BLEDevice, BLEHIDDevice};
use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    timer::EspTaskTimerService,
};

pub mod input;
pub mod key;
mod led;

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

    let ble_server = ble_device.get_server();
    let ble_hid = BLEHIDDevice::new(ble_server);

    let ble_advertising = ble_device.get_advertising();
    ble_advertising.lock().scan_response(false).set_data(
        BLEAdvertisementData::new()
            .name("ESP32 Keyboard")
            .appearance(0x03C1)
            .add_service_uuid(ble_hid.hid_service().lock().uuid()),
    )?;
    ble_advertising.lock().start()?;

    let led_driver = led::Driver::new(Led::new(peripherals.pins.gpio6)?, timer_svc.timer_async()?);
    let input_device = input::Device::new(
        ble_hid,
        timer_svc.timer_async()?,
        [
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
        ],
    )?;

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        futures::try_join!(
            led_driver.long_blink_every(Duration::from_millis(300)),
            input_device.start()
        )
    })?;

    Ok(())
}
