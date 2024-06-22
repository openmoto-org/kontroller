//! Abstractions to build a controller layout.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError, timer::EspAsyncTimer};
use futures::{channel::mpsc::Sender, SinkExt};

use crate::{
    hid,
    key::{self, Key as HwKey},
    keycode::KeyCode,
};

/// The type of a single key in the Layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Up button key from the directional pad.
    Up,
    /// Down button key from the directional pad.
    Down,
    /// Left button key from the directional pad.
    Left,
    /// Right button key from the directional pad.
    Right,
    /// The enter key.
    Enter,
    /// The first function key.
    Fn1,
    /// The second function key.
    Fn2,
    /// The third function key.
    Fn3,
}

/// Represents the layout of the Controller.
pub struct Reporter<'d> {
    keys: HashMap<Key, HwKey<'d>>,
}

impl<'d> Reporter<'d> {
    /// TODO
    ///
    /// # Errors
    pub fn new(
        keys: impl IntoIterator<Item = (Key, impl Into<AnyIOPin>)>,
    ) -> Result<Self, EspError> {
        Ok(Self {
            keys: keys
                .into_iter()
                .map(|(key_type, pin)| Ok((key_type, HwKey::try_from(pin)?)))
                .collect::<Result<HashMap<Key, HwKey<'d>>, EspError>>()?,
        })
    }

    /// # Errors
    ///
    pub async fn start<Clk>(
        &mut self,
        clock: Clk,
        timer: &mut EspAsyncTimer,
        mut tx: Sender<hid::Report>,
    ) -> anyhow::Result<()>
    where
        Clk: Fn() -> Instant,
    {
        loop {
            // TODO(ar3s3ru): inject this value through some configuration.
            timer.after(Duration::from_micros(100)).await?;

            let pressed_keys = self.report_pressed_keys(clock());
            if pressed_keys.is_empty() {
                continue;
            }

            let mut report = hid::Report::default();

            for (i, evt) in pressed_keys.iter().enumerate() {
                report.keycodes[i] = match evt {
                    (_, key::Event::Up) => 0x00,
                    (Key::Up, key::Event::Down) => KeyCode::UP as u8,
                    (Key::Down, key::Event::Down) => KeyCode::Down as u8,
                    (Key::Left, key::Event::Down) => KeyCode::Left as u8,
                    (Key::Right, key::Event::Down) => KeyCode::Right as u8,
                    (Key::Enter, key::Event::Down) => KeyCode::Enter as u8,
                    (Key::Fn1, key::Event::Down) => KeyCode::F7 as u8,
                    (Key::Fn2, key::Event::Down) => KeyCode::F6 as u8,
                    (Key::Fn3, key::Event::Down) => KeyCode::F5 as u8,
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
    pub fn report_pressed_keys(&mut self, now: Instant) -> Vec<(Key, key::Event)> {
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
