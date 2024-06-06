//! ESP32-based firmware to power an openmoto `kontroller`.

#![feature(array_try_map)]

use std::time::{Duration, Instant};

use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    timer::EspTaskTimerService,
};

pub mod key;
pub mod layout;

use layout::{DirectionalPadKey, KeyType, Layout};

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::debug!("Initializing peripherals...");

    let peripherals = Peripherals::take()?;
    let timer_svc = EspTaskTimerService::new()?;

    let mut layout = Layout::build([
        (
            KeyType::DirectionalPad(DirectionalPadKey::Up),
            peripherals.pins.gpio0.downgrade(),
        ),
        (
            KeyType::DirectionalPad(DirectionalPadKey::Left),
            peripherals.pins.gpio1.downgrade(),
        ),
        (
            KeyType::DirectionalPad(DirectionalPadKey::Right),
            peripherals.pins.gpio2.downgrade(),
        ),
        (
            KeyType::DirectionalPad(DirectionalPadKey::Down),
            peripherals.pins.gpio3.downgrade(),
        ),
    ])?;

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        let mut async_timer = timer_svc.timer_async()?;

        loop {
            async_timer.after(Duration::from_micros(500)).await?;

            let now = Instant::now();
            for (kt, event) in layout.report(now) {
                match event {
                    None => (),
                    Some(key::Event::Up) => log::info!("key {kt:?} up"),
                    Some(key::Event::Down) => log::info!("key {kt:?} down"),
                }
            }

            let delay_us = Instant::elapsed(&now).as_micros();
            log::debug!("Processing took {delay_us}us");
        }
    })
}
