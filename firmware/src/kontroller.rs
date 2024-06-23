//! Abstractions to build a controller layout.

use std::collections::HashMap;

use embassy_time::{Duration, Instant, Timer};
use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError};
use futures::{channel::mpsc::Sender, SinkExt};

use crate::{
    hid,
    key::{self, Key as HwKey},
    proto::kontroller::{hid::v1::KeyCode, v1::Button},
};

/// Represents the layout of the Controller.
pub struct Reporter<'d> {
    keys: HashMap<Button, HwKey<'d>>,
}

impl<'d> Reporter<'d> {
    /// TODO
    ///
    /// # Errors
    pub fn new(
        keys: impl IntoIterator<Item = (Button, impl Into<AnyIOPin>)>,
    ) -> Result<Self, EspError> {
        Ok(Self {
            keys: keys
                .into_iter()
                .map(|(key_type, pin)| Ok((key_type, HwKey::try_from(pin)?)))
                .collect::<Result<HashMap<Button, HwKey<'d>>, EspError>>()?,
        })
    }

    /// # Errors
    ///
    pub async fn start<Clk>(
        &mut self,
        clock: Clk,
        mut tx: Sender<hid::Report>,
    ) -> anyhow::Result<()>
    where
        Clk: Fn() -> Instant,
    {
        loop {
            // TODO(ar3s3ru): inject this value through some configuration.
            Timer::after(Duration::from_micros(100)).await;

            let pressed_keys = self.report_pressed_keys(clock());
            if pressed_keys.is_empty() {
                continue;
            }

            let mut report = hid::Report::default();

            for (i, evt) in pressed_keys.iter().enumerate() {
                report.keycodes[i] = match evt {
                    (_, key::Event::Up) | (Button::Unspecified, _) => KeyCode::Unspecified as u8,
                    (Button::Up, key::Event::Down) => KeyCode::Up as u8,
                    (Button::Down, key::Event::Down) => KeyCode::Down as u8,
                    (Button::Left, key::Event::Down) => KeyCode::Left as u8,
                    (Button::Right, key::Event::Down) => KeyCode::Right as u8,
                    (Button::Enter, key::Event::Down) => KeyCode::Enter as u8,
                    (Button::Fn1, key::Event::Down) => KeyCode::F7 as u8,
                    (Button::Fn2, key::Event::Down) => KeyCode::F6 as u8,
                    (Button::Fn3, key::Event::Down) => KeyCode::F5 as u8,
                };
            }

            tx.send(report).await?;
        }
    }

    /// TODO
    ///
    /// # Errors
    ///
    /// # Panics
    pub fn report_pressed_keys(&mut self, now: Instant) -> Vec<(Button, key::Event)> {
        self.keys
            .iter_mut()
            .map(|(kt, key)| (kt, key.update(now)))
            .filter(|(_, evt)| evt.is_some())
            .map(|(kt, evt)| {
                log::info!("{evt:?} {kt:?}");
                (*kt, evt.unwrap())
            })
            .collect()
    }
}
