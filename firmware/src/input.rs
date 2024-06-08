//! Abstractions to build a controller layout.

use std::{
    array,
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::{Duration, Instant},
};

use esp32_nimble::{
    hid::{
        COLLECTION, END_COLLECTION, HIDINPUT, HIDOUTPUT, LOGICAL_MAXIMUM, LOGICAL_MINIMUM,
        REPORT_COUNT, REPORT_ID, REPORT_SIZE, USAGE, USAGE_MAXIMUM, USAGE_MINIMUM, USAGE_PAGE,
    },
    utilities::mutex::Mutex as Esp32NimbleMutex,
    BLECharacteristic, BLEHIDDevice,
};
use esp_idf_svc::{hal::gpio::AnyIOPin, sys::EspError, timer::EspAsyncTimer};

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

// FIXME(ar3s3ru): check if everything is really needed? E.g. keys that will not be outputted by the device.
// Source: https://github.com/taks/esp32-nimble/blob/main/examples/ble_keyboard.rs
const KEYBOARD_ID: u8 = 0x01;
const HID_REPORT_DESCRIPTOR: &[u8] = esp32_nimble::hid::hid!(
    (USAGE_PAGE, 0x01),       // USAGE_PAGE (Generic Desktop Ctrls)
    (USAGE, 0x06),            // USAGE (Keyboard)
    (COLLECTION, 0x01),       // COLLECTION (Application)
    (REPORT_ID, KEYBOARD_ID), //   REPORT_ID (1)
    (USAGE_PAGE, 0x07),       //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0xE0),    //   USAGE_MINIMUM (0xE0)
    (USAGE_MAXIMUM, 0xE7),    //   USAGE_MAXIMUM (0xE7)
    (LOGICAL_MINIMUM, 0x00),  //   LOGICAL_MINIMUM (0)
    (LOGICAL_MAXIMUM, 0x01),  //   Logical Maximum (1)
    (REPORT_SIZE, 0x01),      //   REPORT_SIZE (1)
    (REPORT_COUNT, 0x08),     //   REPORT_COUNT (8)
    (HIDINPUT, 0x02), //   INPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x01), //   REPORT_COUNT (1) ; 1 byte (Reserved)
    (REPORT_SIZE, 0x08), //   REPORT_SIZE (8)
    (HIDINPUT, 0x01), //   INPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (REPORT_COUNT, 0x05), //   REPORT_COUNT (5) ; 5 bits (Num lock, Caps lock, Scroll lock, Compose, Kana)
    (REPORT_SIZE, 0x01),  //   REPORT_SIZE (1)
    (USAGE_PAGE, 0x08),   //   USAGE_PAGE (LEDs)
    (USAGE_MINIMUM, 0x01), //   USAGE_MINIMUM (0x01) ; Num Lock
    (USAGE_MAXIMUM, 0x05), //   USAGE_MAXIMUM (0x05) ; Kana
    (HIDOUTPUT, 0x02), //   OUTPUT (Data,Var,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x01), //   REPORT_COUNT (1) ; 3 bits (Padding)
    (REPORT_SIZE, 0x03), //   REPORT_SIZE (3)
    (HIDOUTPUT, 0x01), //   OUTPUT (Const,Array,Abs,No Wrap,Linear,Preferred State,No Null Position,Non-volatile)
    (REPORT_COUNT, 0x06), //   REPORT_COUNT (6) ; 6 bytes (Keys)
    (REPORT_SIZE, 0x08), //   REPORT_SIZE(8)
    (LOGICAL_MINIMUM, 0x00), //   LOGICAL_MINIMUM(0)
    (LOGICAL_MAXIMUM, 0x65), //   LOGICAL_MAXIMUM(0x65) ; 101 keys
    (USAGE_PAGE, 0x07), //   USAGE_PAGE (Kbrd/Keypad)
    (USAGE_MINIMUM, 0x00), //   USAGE_MINIMUM (0)
    (USAGE_MAXIMUM, 0x65), //   USAGE_MAXIMUM (0x65)
    (HIDINPUT, 0x00),  //   INPUT (Data,Array,Abs,No Wrap,Linear,Preferred State,No Null Position)
    (END_COLLECTION),  // END_COLLECTION
);

/// Default poll interval used in [`Device::start`] to send HID report updates
/// to the connected host.
pub const DEFAULT_DEVICE_POLL_INTERVAL: Duration = Duration::from_millis(1);

/// Contains all the configuration aspects for a [`Device`] behavior.
pub struct DeviceConfig {
    /// The interval at which the [`Device`] HID report should be collected and sent
    /// to the connected host.
    pub poll_interval: Duration,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            poll_interval: DEFAULT_DEVICE_POLL_INTERVAL,
        }
    }
}

struct BLEInputCharacteristic(Arc<Esp32NimbleMutex<BLECharacteristic>>);

impl Deref for BLEInputCharacteristic {
    type Target = Arc<Esp32NimbleMutex<BLECharacteristic>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BLEInputCharacteristic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Represents the layout of the Controller.
pub struct Device<'d> {
    ble_input: BLEInputCharacteristic,
    timer: EspAsyncTimer,
    clock: Box<dyn Fn() -> Instant + Send + Sync>,
    keys: HashMap<Key, HwKey<'d>>,
    config: DeviceConfig,
}

impl<'d> Device<'d> {
    /// TODO
    ///
    /// # Errors
    pub fn new(
        mut ble_hid: BLEHIDDevice,
        timer: EspAsyncTimer,
        keys: impl IntoIterator<Item = (Key, impl Into<AnyIOPin>)>,
    ) -> Result<Self, EspError> {
        ble_hid.manufacturer("openmoto.org");
        ble_hid.pnp(0x02, 0x05ac, 0x820a, 0x0210);
        ble_hid.hid_info(0x00, 0x01);
        ble_hid.set_battery_level(100);
        ble_hid.report_map(HID_REPORT_DESCRIPTOR);

        Ok(Self {
            ble_input: BLEInputCharacteristic(ble_hid.input_report(KEYBOARD_ID)),
            timer,
            clock: Box::new(Instant::now),
            keys: keys
                .into_iter()
                .map(|(key_type, pin)| Ok((key_type, HwKey::try_from(pin)?)))
                .collect::<Result<HashMap<Key, HwKey<'d>>, EspError>>()?,
            config: DeviceConfig::default(),
        })
    }

    /// TODO
    ///
    /// # Errors
    pub async fn start(mut self) -> Result<(), EspError> {
        loop {
            self.timer.after(self.config.poll_interval).await?;

            let now = (self.clock)();
            self.send_report(now);

            let delay_us = ((self.clock)() - now).as_micros();
            log::debug!("Processing took {delay_us}us");
        }
    }
}

#[allow(dead_code)] // NOTE: false positive - it is used by esp32-nimble.
#[derive(Debug)]
#[repr(packed)]
struct KeyReport {
    modifiers: u8,
    reserved: u8,
    keys: [u8; 6],
}

impl<'d> Device<'d> {
    fn send_report(&mut self, now: Instant) {
        let report = self.generate_report(now);
        log::debug!("Report: {report:?}");
        self.ble_input.lock().set_from(&report).notify();
    }

    fn generate_report(&mut self, now: Instant) -> KeyReport {
        let mut keys_report: [u8; 6] = Default::default();

        let pressed_keys = self
            .keys
            .iter_mut()
            .map(|(kt, key)| (kt.ascii_keycode(), key.update(now)))
            .filter(|(_, evt)| *evt == Some(key::Event::Down))
            .map(|(kc, _)| kc)
            .enumerate();

        for (i, keycode) in pressed_keys {
            keys_report[i] = keycode;
        }

        KeyReport {
            modifiers: 0,
            reserved: 0,
            keys: keys_report,
        }
    }
}
