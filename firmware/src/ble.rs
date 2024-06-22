use std::{sync::Arc, time::Duration};

use esp32_nimble::{
    enums::{AuthReq, SecurityIOCap},
    utilities::mutex::Mutex,
    BLEAdvertisementData, BLECharacteristic, BLEDevice, BLEError, BLEHIDDevice, BLEServer,
};
use esp_idf_svc::{
    hal::{delay, timer::Timer},
    sys::EspError,
    timer::EspAsyncTimer,
};
use futures::{channel::mpsc::Receiver, StreamExt};
use log::{info, warn};
use usbd_hid::descriptor::SerializedDescriptor;

use crate::hid;

pub type HidWriter = Arc<Mutex<BLECharacteristic>>;
pub type HidReader = Arc<Mutex<BLECharacteristic>>;

#[derive(Debug, Clone)]
pub struct Config {
    pub device_name: &'static str,
}

pub struct Server {
    device: &'static mut BLEDevice,
    #[allow(clippy::struct_field_names)]
    server: &'static mut BLEServer,
    input_keyboard: HidWriter,
}

impl Server {
    pub fn new(config: &Config) -> Result<Self, BLEError> {
        BLEDevice::set_device_name(config.device_name)?;

        let device = BLEDevice::take();

        device
            .security()
            .set_auth(AuthReq::all())
            .set_io_cap(SecurityIOCap::NoInputNoOutput)
            .resolve_rpa();

        let server = device.get_server();

        server.on_connect(|_, r| {
            info!("connection established: {r:?}");
        });

        server.on_disconnect(|t, r| match r {
            Ok(()) => info!("connection closed: {t:?}"),
            Err(err) => warn!("connection aborted, cause: (code: {} {err}", err.code()),
        });

        let mut hid_device = BLEHIDDevice::new(server);

        let input_keyboard = hid_device.input_report(hid::ReportType::Keyboard as u8);

        hid_device.manufacturer("test");
        hid_device.pnp(
            0x02,
            hid::APPLE_INC_VENDOR_ID,
            hid::APPLE_BLUETOOTH_HID_KEYBOARD_PRODUCT_ID,
            0x0210,
        );
        hid_device.set_battery_level(100);
        hid_device.hid_info(0x00, 0x03);
        hid_device.report_map(hid::Report::desc());

        let advertising = device.get_advertising();

        advertising.lock().scan_response(false).set_data(
            BLEAdvertisementData::new()
                .name(config.device_name)
                .appearance(hid::BLE_APPEARANCE_KEYBOARD)
                .add_service_uuid(hid_device.hid_service().lock().uuid()),
        )?;

        Ok(Self {
            device,
            server,
            input_keyboard,
        })
    }

    pub async fn start(
        &mut self,
        timer: &mut EspAsyncTimer,
        mut rx: Receiver<hid::Report>,
    ) -> anyhow::Result<()> {
        loop {
            self.device.get_advertising().lock().start()?;

            self.wait_for_connection(timer).await?;

            self.device.get_advertising().lock().stop()?;

            futures::try_join!(
                self.listen_for_reports(&mut rx),
                self.wait_for_disconnection(timer)
            )?;
        }
    }

    async fn wait_for_connection(&self, timer: &mut EspAsyncTimer) -> anyhow::Result<()> {
        loop {
            timer.after(Duration::from_millis(100)).await?;
            if self.server.connected_count() > 0 {
                return Ok(());
            }
        }
    }

    async fn listen_for_reports(&self, rx: &mut Receiver<hid::Report>) -> anyhow::Result<()> {
        while let Some(report) = rx.next().await {
            info!("report received: {report:?}");
            self.send_report(&report);
        }

        Ok(())
    }

    async fn wait_for_disconnection(&self, timer: &mut EspAsyncTimer) -> anyhow::Result<()> {
        loop {
            timer.after(Duration::from_millis(500)).await?;
            if self.server.connected_count() == 0 {
                return Ok(());
            }
        }
    }

    fn send_report<T: Sized>(&self, report: &T) {
        self.input_keyboard.lock().set_from(report).notify();
        delay::Ets::delay_ms(7);
    }
}
