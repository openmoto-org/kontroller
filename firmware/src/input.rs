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
pub struct Device<'d> {
    keys: HashMap<Key, HwKey<'d>>,
}

impl<'d> Device<'d> {
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
    pub fn report(&mut self, now: Instant) -> HashMap<Key, Option<key::Event>> {
        self.keys
            .iter_mut()
            .map(|(kt, key)| (*kt, key.update(now)))
            .collect()
    }
}
