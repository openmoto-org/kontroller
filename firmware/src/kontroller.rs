use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use esp32_nimble::{
    enums::{AuthReq, SecurityIOCap},
    hid::{
        COLLECTION, END_COLLECTION, HIDINPUT, HIDOUTPUT, LOGICAL_MAXIMUM, LOGICAL_MINIMUM,
        REPORT_COUNT, REPORT_ID, REPORT_SIZE, USAGE, USAGE_MAXIMUM, USAGE_MINIMUM, USAGE_PAGE,
    },
    utilities::mutex::Mutex as Esp32NimbleMutex,
    BLEAdvertisementData, BLEAdvertising, BLECharacteristic, BLEDevice, BLEHIDDevice, BLEServer,
};
use esp_idf_svc::timer::{EspAsyncTimer, EspTaskTimerService};

use crate::{input, led};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    NotStarted,
    Unpaired,
    Paired,
}

pub struct Subsystems<'d> {
    pub ble_device: &'d mut BLEDevice,
    pub led_blinker: led::Blinker<'d>,
    pub input_reporter: input::Reporter<'d>,
}

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

pub struct Kontroller<'d> {
    status: Arc<RwLock<Status>>,
    timer: EspTaskTimerService,
    subsystems: Subsystems<'d>,
}

// Source: <https://developer.nordicsemi.com/nRF5_SDK/nRF51_SDK_v4.x.x/doc/html/group___b_l_e___a_p_p_e_a_r_a_n_c_e_s.html#gac08ceb7b199eceefc4650399a3a7ff75>
const BLE_APPEARANCE_HID_KEYBOARD: u16 = 0x03c1;
// Source: <https://the-sz.com/products/usbid/index.php?v=0x05AC&p=0x820A>
const USB_HID_APPLE_INC_VENDOR_ID: u16 = 0x05ac;
const USB_HID_APPLE_BLUETOOTH_HID_KEYBOARD_PRODUCT_ID: u16 = 0x820a;

impl<'d> Kontroller<'d> {
    pub fn new(timer: EspTaskTimerService, subsystems: Subsystems<'d>) -> Self {
        Self {
            timer,
            subsystems,
            status: Arc::new(RwLock::new(Status::NotStarted)),
        }
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        let input_reporter = &mut self.subsystems.input_reporter;
        let led_blinker = self.subsystems.led_blinker;

        let ble_device = self.subsystems.ble_device;

        BLEDevice::set_device_name("ESP32 Keyboard")?;

        ble_device
            .security()
            .set_auth(AuthReq::all())
            .set_io_cap(SecurityIOCap::NoInputNoOutput)
            .resolve_rpa();

        let ble_server = ble_device.get_server();
        let ble_advertising = ble_device.get_advertising();
        let mut ble_hid = BLEHIDDevice::new(ble_server);

        ble_hid.manufacturer("openmoto.org");
        ble_hid.pnp(
            0x02,
            USB_HID_APPLE_INC_VENDOR_ID,
            USB_HID_APPLE_BLUETOOTH_HID_KEYBOARD_PRODUCT_ID,
            0x0210,
        );
        ble_hid.hid_info(0x00, 0x01); // Country: not supported.
        ble_hid.set_battery_level(100);
        ble_hid.report_map(HID_REPORT_DESCRIPTOR);

        let ble_hid_input_characteristic = ble_hid.input_report(KEYBOARD_ID);
        let ble_uuid = ble_hid.hid_service().lock().uuid();

        ble_advertising.lock().scan_response(true).set_data(
            BLEAdvertisementData::new()
                .name("ESP32 Keyboard")
                .appearance(BLE_APPEARANCE_HID_KEYBOARD)
                .add_service_uuid(ble_uuid),
        )?;

        // Updates the status to Status::Unpaired and drops the lock.
        *(self.status.write().unwrap()) = Status::Unpaired;

        futures::try_join!(
            Self::handle_led_status(self.timer.timer_async()?, self.status.clone(), &led_blinker,),
            Self::handle_ble_connection(self.status.clone(), ble_server, ble_advertising),
            Self::handle_input_detection(
                self.timer.timer_async()?,
                ble_hid_input_characteristic.clone(),
                &led_blinker,
                input_reporter,
            )
        )?;

        Ok(())
    }
}

impl<'d> Kontroller<'d> {
    pub async fn handle_led_status(
        mut timer: EspAsyncTimer,
        status: Arc<RwLock<Status>>,
        led_blinker: &led::Blinker<'d>,
    ) -> anyhow::Result<()> {
        let timer: &mut EspAsyncTimer = timer.every(Duration::from_millis(500))?;

        loop {
            timer.tick().await?;

            let status = {
                let guard = status.read().unwrap();
                *guard
            };

            match status {
                Status::NotStarted => continue,
                Status::Unpaired => led_blinker.short_blink().await?,
                Status::Paired => led_blinker.long_blink().await?,
            }
        }
    }

    #[allow(clippy::unused_async)]
    pub async fn handle_ble_connection(
        status: Arc<RwLock<Status>>,
        ble_server: &mut BLEServer,
        ble_advertising: &Esp32NimbleMutex<BLEAdvertising>,
    ) -> anyhow::Result<()> {
        let status_on_connect = status.clone();
        ble_server.on_connect(move |_, desc| {
            log::info!("BLE client connected: {desc:?}");
            *(status_on_connect.write().unwrap()) = Status::Paired;
        });

        let status_on_disconnect = status.clone();
        ble_server.on_disconnect(move |desc, reason| {
            let reason = reason.unwrap_err();
            log::info!("BLE client disconnected: {desc:?}, reason: {reason}");
            *(status_on_disconnect.write().unwrap()) = Status::Unpaired;
        });

        ble_advertising.lock().start()?;

        log::info!("exiting...");

        Ok(())
    }
}

#[allow(dead_code)] // NOTE: false positive - it is used by esp32-nimble.
#[derive(Debug)]
#[repr(packed)]
struct KeyReport {
    modifiers: u8,
    reserved: u8,
    keys: [u8; 6], // NOTE: this is coming from esp32-nimble.
}

impl KeyReport {
    fn from_pressed_keys(keys: &[input::Key]) -> Self {
        let mut result: [u8; 6] = Default::default();

        for (i, key) in keys.iter().enumerate() {
            result[i] = match key {
                input::Key::DirectionalPad(input::DirectionalPad::Up) => 0xDA,
                input::Key::DirectionalPad(input::DirectionalPad::Left) => 0xD8,
                input::Key::DirectionalPad(input::DirectionalPad::Right) => 0xD7,
                input::Key::DirectionalPad(input::DirectionalPad::Down) => 0xD9,
                input::Key::Enter => 0xB0,
                input::Key::Function => 0xC6,
            };
        }

        Self {
            modifiers: 0,
            reserved: 0,
            keys: result,
        }
    }
}

impl<'d> Kontroller<'d> {
    async fn handle_input_detection(
        mut timer: EspAsyncTimer,
        ble_hid_input_characteristic: Arc<Esp32NimbleMutex<BLECharacteristic>>,
        led_blinker: &led::Blinker<'d>,
        input_reporter: &mut input::Reporter<'d>,
    ) -> anyhow::Result<()> {
        let timer: &mut EspAsyncTimer = timer.every(Duration::from_millis(1))?;

        loop {
            timer.tick().await?;

            let pressed_keys = input_reporter.report_pressed_keys(Instant::now());
            if pressed_keys.is_empty() {
                continue;
            }

            let key_report = KeyReport::from_pressed_keys(&pressed_keys);

            ble_hid_input_characteristic
                .lock()
                .set_from(&key_report)
                .notify();

            led_blinker.short_blink().await?;
        }
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
