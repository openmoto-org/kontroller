//! ESP32-based firmware to power an openmoto `kontroller`.

#![feature(array_try_map)]

use std::time::{Duration, Instant};

use esp_idf_svc::{
    hal::{gpio::IOPin, peripherals::Peripherals, task},
    sys::EspError,
    timer::EspTaskTimerService,
};

pub mod key;
pub mod layout;
mod led;

use layout::{DirectionalPadKey, KeyType, Layout};
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

    let led_driver = led::Driver::new(Led::new(peripherals.pins.gpio6)?, timer_svc.timer_async()?);
    let layout = Layout::build([
        (
            DirectionalPadKey::Up.into(),
            peripherals.pins.gpio0.downgrade(),
        ),
        (
            DirectionalPadKey::Left.into(),
            peripherals.pins.gpio1.downgrade(),
        ),
        (
            DirectionalPadKey::Right.into(),
            peripherals.pins.gpio2.downgrade(),
        ),
        (
            DirectionalPadKey::Down.into(),
            peripherals.pins.gpio3.downgrade(),
        ),
        (KeyType::Enter, peripherals.pins.gpio4.downgrade()),
        (KeyType::Function, peripherals.pins.gpio5.downgrade()),
    ])?;

    log::debug!("Peripherals fully initialized");

    task::block_on(async {
        futures::try_join!(
            led_driver.long_blink_every(Duration::from_millis(300)),
            keys_driver(layout, &led_driver, &timer_svc),
        )
    })?;

    Ok(())
}

async fn keys_driver(
    mut layout: Layout<'_>,
    led_driver: &led::Driver<'_>,
    timer_svc: &EspTaskTimerService,
) -> Result<(), EspError> {
    let mut async_timer = timer_svc.timer_async()?;

    loop {
        async_timer.after(Duration::from_micros(500)).await?;

        let now = Instant::now();
        for (kt, event) in layout.report(now) {
            match event {
                None => (),
                Some(key::Event::Up) => {
                    log::info!("key {kt:?} up");
                    led_driver.off().await?;
                }
                Some(key::Event::Down) => {
                    log::info!("key {kt:?} down");
                    led_driver.on().await?;
                }
            }
        }

        let delay_us = Instant::elapsed(&now).as_micros();
        log::debug!("Processing took {delay_us}us");
    }
}
