//! Abstractions to build a controller layout.

use std::collections::HashMap;

use embassy_time::{Duration, Instant, Timer};
use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError};
use futures::{channel::mpsc::Sender, SinkExt};

use crate::{
    hid,
    key::{self, Key as HwKey},
    proto::kontroller::{
        hid::v1::KeyCode,
        v1::{keymap::Entry, Button, Keymap, Konfiguration},
    },
};

pub fn make_keymap(it: impl IntoIterator<Item = (Button, KeyCode)>) -> Keymap {
    Keymap {
        entries: it
            .into_iter()
            .map(|(button, key_code)| Entry {
                button: button.into(),
                key_code: key_code.into(),
            })
            .collect(),
    }
}

/// Represents the layout of the Controller.
pub struct Kontroller<'d> {
    keys: HashMap<Button, HwKey<'d>>,
    config: Konfiguration,
}

impl<'d> Kontroller<'d> {
    /// TODO
    ///
    /// # Errors
    pub fn new(
        keys: impl IntoIterator<Item = (Button, impl Into<AnyIOPin>)>,
        config: Konfiguration,
    ) -> Result<Self, EspError> {
        Ok(Self {
            config,
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
            Timer::after(Duration::from_micros(
                self.config.buttons_poll_interval_micros,
            ))
            .await;

            let pressed_keys = self.report_pressed_keys(clock());
            if pressed_keys.is_empty() {
                continue;
            }

            let mut report = hid::Report::default();

            for (i, evt) in pressed_keys.iter().enumerate() {
                report.keycodes[i] = match evt {
                    (_, key::Event::Up) | (Button::Unspecified, _) => KeyCode::Unspecified as u8,
                    (button, key::Event::Down) => self
                        .config
                        .keymap
                        .as_ref()
                        .and_then(|keymap| {
                            keymap
                                .entries
                                .iter()
                                .find(|entry| entry.button() == *button)
                        })
                        .map_or(KeyCode::Unspecified, Entry::key_code)
                        as u8,
                }
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
