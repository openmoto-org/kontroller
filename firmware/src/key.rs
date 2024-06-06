//! Module containing logical abstraction for a physical, debounced [`Key`].

use std::time::{Duration, Instant};

use esp_idf_svc::{
    hal::gpio::{AnyIOPin, Input, PinDriver, Pull},
    sys::EspError,
};

/// Default debounce timeout used before triggering an [`Event::Down`] when the [`Key`]
/// is pressed.
pub const DEFAULT_DEBOUNCE_TIMEOUT: Duration = Duration::from_micros(900);

/// Default release timeout before triggering an [`Event::Up`] when the [`Key`]
/// is depressed.
pub const DEFAULT_RELEASE_TIMEOUT: Duration = Duration::from_millis(1);

/// Default hold timeout used by the [`Key`] to detect when it is being long-pressed.
pub const DEFAULT_HOLD_TIMEOUT: Duration = Duration::from_millis(500);

/// Default hold repeat timeout, used to trigger an additional [`Event::Down`] when the [`Key`]
/// is still pressed down.
pub const DEFAULT_HOLD_REPEAT_TIMEOUT: Duration = Duration::from_millis(100);

/// Configuration for the [`Key`] state machine transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// Debounce timeout is used to trigger an [`Event::Down`] when a [`Key`] is pressed
    /// from a depressed state.
    pub debounce: Duration,
    /// Release timeout is used to trigger an [`Event::Up`] when a [`Key`] is depressed
    /// from a pressed state.
    pub release: Duration,
    /// Hold timeout is used to detect long-presses on the [`Key`].
    pub hold: Duration,
    /// Repeat timeout used to trigger consecutive [`Event::Down`] when the [`Key`]
    /// is in [`State::Held`].
    pub hold_repeat: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            debounce: DEFAULT_DEBOUNCE_TIMEOUT,
            release: DEFAULT_RELEASE_TIMEOUT,
            hold: DEFAULT_HOLD_TIMEOUT,
            hold_repeat: DEFAULT_HOLD_REPEAT_TIMEOUT,
        }
    }
}

/// All different states for the [`Key`] state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Released,
    Down(Instant),
    Pressed(Instant),
    Held(Instant),
    Up(Instant),
}

/// All possible events that can be detected by the [`Key`] state machine
/// when performing a new signal pin scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The [`Key`] has been depressed.
    Up,
    /// The [`Key`] has been pressed, or it is still being pressed.
    Down,
}

/// Logical representation of a physical key, or button, that is connected
/// to a microcontroller pin using pull-up resistors (or no resistors at all).
///
/// Use [`Key::try_from`] to build a new [`Key`] instance using the default
/// [`Config`] value.
pub struct Key<'d> {
    pin: PinDriver<'d, AnyIOPin, Input>,
    state: State,
    config: Config,
}

impl<'d> Key<'d> {
    /// Builds a [`Key`] instance from a given GPIO pin.
    ///
    /// # Errors
    ///
    /// The method fails when it's unable to create and setup correctly a [`PinDriver`] for the specified GPIO pin.
    pub fn try_from(pin: impl Into<AnyIOPin>) -> Result<Self, EspError> {
        let mut pin_driver = PinDriver::input(pin.into())?;
        pin_driver.set_pull(Pull::Up)?;

        Ok(Self {
            pin: pin_driver,
            config: Config::default(),
            state: State::Released,
        })
    }

    /// Updates the internal state of the [`Key`] based on the current timestamp.
    ///
    /// This method should be called from within a `loop`, either on the main microcontroller
    /// thread or on a dedicated task (sync or async).
    ///
    /// Returns an optional [`Event`] if the state machine transition
    /// has detected one.
    pub fn update(&mut self, now: Instant) -> Option<Event> {
        match self.state {
            State::Released if self.pin.is_low() => {
                self.state = State::Down(now);
                None
            }
            State::Released => None,
            State::Down(last) => {
                if self.pin.is_low() && self.debounced(now, last) {
                    self.state = State::Pressed(now);
                    return Some(Event::Down);
                }

                if self.pin.is_high() {
                    self.state = State::Up(now);
                }

                None
            }
            State::Pressed(last) => {
                if self.pin.is_low() && self.held(now, last) {
                    self.state = State::Held(now);
                    return Some(Event::Down);
                }

                if self.pin.is_high() {
                    self.state = State::Up(now);
                }

                None
            }
            State::Held(last) => {
                if self.pin.is_low() && self.still_held(now, last) {
                    self.state = State::Held(now);
                    return Some(Event::Down);
                }

                if self.pin.is_high() {
                    self.state = State::Up(now);
                }

                None
            }
            State::Up(last) => {
                if self.pin.is_high() && self.released(now, last) {
                    self.state = State::Released;
                    return Some(Event::Up);
                }

                if self.pin.is_low() && !self.released(now, last) {
                    self.state = State::Down(now);
                }

                None
            }
        }
    }

    fn debounced(&self, now: Instant, last: Instant) -> bool {
        now - last >= self.config.debounce
    }

    fn released(&self, now: Instant, last: Instant) -> bool {
        now - last >= self.config.release
    }

    fn held(&self, now: Instant, last: Instant) -> bool {
        now - last >= self.config.hold
    }

    fn still_held(&self, now: Instant, last: Instant) -> bool {
        now - last >= self.config.hold_repeat
    }
}
