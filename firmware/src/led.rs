use std::{ops::Deref, rc::Rc, time::Duration};

use esp_idf_svc::{
    hal::gpio::{AnyIOPin, InputOutput, PinDriver},
    sys::EspError,
    timer::EspAsyncTimer,
};
use futures::lock::Mutex;

pub struct Led<'d> {
    pin: Rc<Mutex<PinDriver<'d, AnyIOPin, InputOutput>>>,
}

impl<'d> Led<'d> {
    pub fn new(pin: impl Into<AnyIOPin>) -> Result<Self, EspError> {
        Ok(Self {
            pin: Rc::new(Mutex::new(PinDriver::input_output(pin.into())?)),
        })
    }

    pub async fn on(&self) -> Result<(), EspError> {
        let mut pin_mut = self.pin.lock().await;

        pin_mut.set_high()?;
        pin_mut.wait_for_high().await
    }

    pub async fn off(&self) -> Result<(), EspError> {
        let mut pin_mut = self.pin.lock().await;

        pin_mut.set_low()?;
        pin_mut.wait_for_low().await
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
    timer: Mutex<EspAsyncTimer>,
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
        Self {
            led,
            config,
            timer: Mutex::new(timer),
        }
    }

    pub async fn short_blink(&self) -> Result<(), EspError> {
        self.blink(self.config.short_blink_duration).await
    }

    pub async fn long_blink(&self) -> Result<(), EspError> {
        self.blink(self.config.long_blink_duration).await
    }

    pub async fn blink(&self, d: Duration) -> Result<(), EspError> {
        let mut timer = self.timer.lock().await;

        self.led.on().await?;
        timer.after(d).await?;

        self.led.off().await?;
        timer.after(d).await
    }
}
