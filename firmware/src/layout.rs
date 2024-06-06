//! Abstractions to build a controller layout.

use std::{collections::HashMap, time::Instant};

use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError};

use crate::key::{self, Key};

/// The type of a single key in the Layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyType {
    /// A directional pad key.
    DirectionalPad(DirectionalPadKey),
    /// The enter key.
    Enter,
    /// The function key.
    Function,
}

/// All possible types of Directional Pad keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalPadKey {
    /// Arrow up.
    Up,
    /// Arrow down.
    Down,
    /// Arrow left.
    Left,
    /// Arrow right.
    Right,
}

impl From<DirectionalPadKey> for KeyType {
    fn from(value: DirectionalPadKey) -> Self {
        Self::DirectionalPad(value)
    }
}

/// Represents the layout of the Controller.
pub struct Layout<'d> {
    keys: HashMap<KeyType, Key<'d>>,
}

impl<'d> Layout<'d> {
    /// TODO
    ///
    /// # Errors
    pub fn build(
        keys: impl IntoIterator<Item = (KeyType, impl Into<AnyIOPin>)>,
    ) -> Result<Self, EspError> {
        Ok(Self {
            keys: keys
                .into_iter()
                .map(|(key_type, pin)| Ok((key_type, Key::try_from(pin)?)))
                .collect::<Result<HashMap<KeyType, Key<'d>>, EspError>>()?,
        })
    }

    /// TODO
    pub fn report(&mut self, now: Instant) -> HashMap<KeyType, Option<key::Event>> {
        self.keys
            .iter_mut()
            .map(|(kt, key)| (*kt, key.update(now)))
            .collect()
    }
}
