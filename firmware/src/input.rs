//! Abstractions to build a controller layout.

use std::{collections::HashMap, time::Instant};

use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError};

use crate::key::{self, Key as HwKey};

/// The type of a single key in the Layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A directional pad key.
    DirectionalPad(DirectionalPad),
    /// The enter key.
    Enter,
    /// The function key.
    Function,
}

impl Key {
    /// Returns the ASCII keycode for the specified input key, to be sent
    /// to the connected host in the HID report.
    #[must_use]
    pub fn ascii_keycode(&self) -> u8 {
        match *self {
            Key::DirectionalPad(DirectionalPad::Up) => 0xDA,
            Key::DirectionalPad(DirectionalPad::Left) => 0xD8,
            Key::DirectionalPad(DirectionalPad::Right) => 0xD7,
            Key::DirectionalPad(DirectionalPad::Down) => 0xD9,
            Key::Enter => 0xB0,
            Key::Function => 0xC6,
        }
    }
}

/// All possible types of Directional Pad keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalPad {
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
}

impl From<DirectionalPad> for Key {
    fn from(value: DirectionalPad) -> Self {
        Self::DirectionalPad(value)
    }
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

    /// TODO
    ///
    /// # Errors
    pub fn report_pressed_keys(&mut self, now: Instant) -> Vec<Key> {
        self.keys
            .iter_mut()
            .map(|(kt, key)| (kt, key.update(now)))
            .filter(|(_, evt)| *evt == Some(key::Event::Down))
            .map(|(kt, _)| *kt)
            .collect()
    }
}
