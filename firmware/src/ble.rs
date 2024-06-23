use std::sync::Arc;

use embassy_time::{Duration, Timer};
use esp32_nimble::{
    enums::{AuthReq, SecurityIOCap},
    utilities::mutex::Mutex,
    BLEAdvertisementData, BLECharacteristic, BLEDevice, BLEError, BLEHIDDevice, BLEServer,
};
use futures::{channel::mpsc::Receiver, future::Either, StreamExt, TryFutureExt};
use log::{info, warn};
use usbd_hid::descriptor::SerializedDescriptor;

use crate::{hid, led, proto::kontroller::hid::v1::ReportType};

pub type HidWriter = Arc<Mutex<BLECharacteristic>>;

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
    pub fn initialize(config: &Config) -> Result<Self, BLEError> {
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

        let input_keyboard = Self::initialize_hid_keyboard(device, server, config)?;

        Ok(Self {
            device,
            server,
            input_keyboard,
        })
    }

    fn initialize_hid_keyboard(
        device: &mut BLEDevice,
        server: &mut BLEServer,
        config: &Config,
    ) -> Result<HidWriter, BLEError> {
        let mut hid_device = BLEHIDDevice::new(server);

        let input_keyboard = hid_device.input_report(ReportType::Keyboard as u8);

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

        Ok(input_keyboard)
    }

    pub async fn start(
        &mut self,
        mut rx: Receiver<hid::Report>,
        led: &mut led::Blinker<'_>,
    ) -> anyhow::Result<()> {
        loop {
            info!("advertising started");

            self.device.get_advertising().lock().start()?;

            let wait_for_connection = Box::pin(self.wait_for_connection());
            let quickly_blink_led = Box::pin(Self::quickly_blink_led(led));

            futures::future::try_select(wait_for_connection, quickly_blink_led)
                .await
                .map_err(|err| match err {
                    Either::Right((err, _)) | Either::Left((err, _)) => err,
                })?;

            self.device.get_advertising().lock().stop()?;

            info!("advertising stopped");

            let listen_hid_reports = Box::pin(self.listen_for_reports(&mut rx, led));
            let wait_for_disconnection = Box::pin(self.wait_for_disconnection());

            futures::future::try_select(listen_hid_reports, wait_for_disconnection)
                .await
                .map_err(|err| match err {
                    Either::Right((err, _)) | Either::Left((err, _)) => err,
                })?;
        }
    }

    async fn quickly_blink_led(led: &mut led::Blinker<'_>) -> anyhow::Result<()> {
        loop {
            led.short_blink().await?;
            // TODO(ar3s3ru): do not hardcode.
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    async fn wait_for_connection(&self) -> anyhow::Result<()> {
        loop {
            // TODO(ar3s3ru): do not hardcode
            Timer::after(Duration::from_millis(100)).await;
            if self.server.connected_count() > 0 {
                return Ok(());
            }
        }
    }

    async fn listen_for_reports(
        &self,
        rx: &mut Receiver<hid::Report>,
        led: &mut led::Blinker<'_>,
    ) -> anyhow::Result<()> {
        while let Some(report) = rx.next().await {
            info!("report received: {report:?}");

            futures::try_join!(
                self.send_report(&report),
                led.short_blink().map_err(anyhow::Error::from)
            )?;
        }

        Ok(())
    }

    async fn wait_for_disconnection(&self) -> anyhow::Result<()> {
        loop {
            // TODO(ar3s3ru): do not hardcode
            Timer::after(Duration::from_millis(500)).await;
            if self.server.connected_count() == 0 {
                return Ok(());
            }
        }
    }

    async fn send_report<T: Sized>(&self, report: &T) -> anyhow::Result<()> {
        self.input_keyboard.lock().set_from(report).notify();
        Timer::after(Duration::from_millis(7)).await;

        Ok(())
    }
}
