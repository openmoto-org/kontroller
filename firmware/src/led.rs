use std::{ops::Deref, time::Duration};

use esp_idf_svc::{
    hal::gpio::{AnyIOPin, InputOutput, PinDriver},
    sys::EspError,
    timer::EspAsyncTimer,
};

pub struct Led<'d> {
    pin: PinDriver<'d, AnyIOPin, InputOutput>,
}

impl<'d> Led<'d> {
    pub fn new(pin: impl Into<AnyIOPin>) -> Result<Self, EspError> {
        Ok(Self {
            pin: PinDriver::input_output(pin.into())?,
        })
    }

    pub async fn on(&mut self) -> Result<(), EspError> {
        self.pin.set_high()?;
        self.pin.wait_for_high().await
    }

    pub async fn off(&mut self) -> Result<(), EspError> {
        self.pin.set_low()?;
        self.pin.wait_for_low().await
    }
}

pub const DEFAULT_SHORT_BLINK_DURATION: Duration = Duration::from_millis(100);
pub const DEFAULT_LONG_BLINK_DURATION: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverConfig {
    pub short_blink_duration: Duration,
    pub long_blink_duration: Duration,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            short_blink_duration: DEFAULT_SHORT_BLINK_DURATION,
            long_blink_duration: DEFAULT_LONG_BLINK_DURATION,
        }
    }
}

pub struct Blinker<'d> {
    led: Led<'d>,
    timer: EspAsyncTimer,
    config: DriverConfig,
}

impl<'d> Deref for Blinker<'d> {
    type Target = Led<'d>;

    fn deref(&self) -> &Self::Target {
        &self.led
    }
}

impl<'d> Blinker<'d> {
    pub fn new(led: Led<'d>, timer: EspAsyncTimer) -> Self {
        Self::new_with_config(led, timer, DriverConfig::default())
    }

    pub fn new_with_config(led: Led<'d>, timer: EspAsyncTimer, config: DriverConfig) -> Self {
        Self { led, timer, config }
    }

    pub async fn short_blink(&mut self) -> Result<(), EspError> {
        self.blink(self.config.short_blink_duration).await
    }

    pub async fn long_blink(&mut self) -> Result<(), EspError> {
        self.blink(self.config.long_blink_duration).await
    }

    pub async fn blink(&mut self, d: Duration) -> Result<(), EspError> {
        self.led.on().await?;
        self.timer.after(d).await?;

        self.led.off().await?;
        self.timer.after(d).await
    }
}
