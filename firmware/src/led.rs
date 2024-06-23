use std::ops::Deref;

use embassy_time::{Duration, Timer};
use esp_idf_svc::{
    hal::gpio::{AnyIOPin, InputOutput, PinDriver},
    sys::EspError,
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

    pub async fn on(&mut self) -> anyhow::Result<()> {
        self.pin.set_high()?;
        self.pin.wait_for_high().await?;

        Ok(())
    }

    pub async fn off(&mut self) -> anyhow::Result<()> {
        self.pin.set_low()?;
        self.pin.wait_for_low().await?;

        Ok(())
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
    config: DriverConfig,
}

impl<'d> Deref for Blinker<'d> {
    type Target = Led<'d>;

    fn deref(&self) -> &Self::Target {
        &self.led
    }
}

impl<'d> From<Led<'d>> for Blinker<'d> {
    fn from(led: Led<'d>) -> Self {
        Self::new(led, DriverConfig::default())
    }
}

impl<'d> Blinker<'d> {
    pub fn new(led: Led<'d>, config: DriverConfig) -> Self {
        Self { led, config }
    }

    #[allow(dead_code)]
    pub async fn short_blink(&mut self) -> anyhow::Result<()> {
        self.blink(self.config.short_blink_duration).await
    }

    #[allow(dead_code)]
    pub async fn long_blink(&mut self) -> anyhow::Result<()> {
        self.blink(self.config.long_blink_duration).await
    }

    pub async fn blink(&mut self, d: Duration) -> anyhow::Result<()> {
        self.led.on().await?;
        Timer::after(d).await;

        self.led.off().await?;
        Timer::after(d).await;

        Ok(())
    }
}
