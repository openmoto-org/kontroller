// @generated
/// A Kontroller button, in the form of an harware key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Button {
    /// Default value, must not be used.
    Unspecified = 0,
    /// The directional pad up button.
    Up = 1,
    /// The directional pad down button.
    Down = 2,
    /// The directional pad left button.
    Left = 3,
    /// The directional pad right button.
    Right = 4,
    /// The directional pad enter button.
    Enter = 5,
    /// The first function button.
    Fn1 = 6,
    /// The second function button.
    Fn2 = 7,
    /// The third function button.
    Fn3 = 8,
}
impl Button {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Button::Unspecified => "BUTTON_UNSPECIFIED",
            Button::Up => "BUTTON_UP",
            Button::Down => "BUTTON_DOWN",
            Button::Left => "BUTTON_LEFT",
            Button::Right => "BUTTON_RIGHT",
            Button::Enter => "BUTTON_ENTER",
            Button::Fn1 => "BUTTON_FN1",
            Button::Fn2 => "BUTTON_FN2",
            Button::Fn3 => "BUTTON_FN3",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "BUTTON_UNSPECIFIED" => Some(Self::Unspecified),
            "BUTTON_UP" => Some(Self::Up),
            "BUTTON_DOWN" => Some(Self::Down),
            "BUTTON_LEFT" => Some(Self::Left),
            "BUTTON_RIGHT" => Some(Self::Right),
            "BUTTON_ENTER" => Some(Self::Enter),
            "BUTTON_FN1" => Some(Self::Fn1),
            "BUTTON_FN2" => Some(Self::Fn2),
            "BUTTON_FN3" => Some(Self::Fn3),
            _ => None,
        }
    }
}
/// A keymap for the Kontroller, i.e. the list of which HID keycode to apply
/// to a specific physical button press.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Keymap {
    /// All the keymap entries.
    #[prost(message, repeated, tag = "1")]
    pub entries: ::prost::alloc::vec::Vec<keymap::Entry>,
}
/// Nested message and enum types in `Keymap`.
pub mod keymap {
    /// A keymap entry, i.e. the association between one Button and a KeyCode.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Entry {
        /// The physical Button.
        #[prost(enumeration = "super::Button", tag = "1")]
        pub button: i32,
        /// The key code to apply to the physical Button.
        #[prost(enumeration = "super::super::hid::v1::KeyCode", tag = "2")]
        pub key_code: i32,
    }
}
/// A Kontroller configuration.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Konfiguration {
    /// The interval between each polling call for hardware buttons state.
    /// Expressed in microseconds.
    #[prost(uint64, tag = "1")]
    pub buttons_poll_interval_micros: u64,
    /// The keymap for the Kontroller, i.e. which HID keycodes to apply
    /// to a physical Button press.
    #[prost(message, optional, tag = "2")]
    pub keymap: ::core::option::Option<Keymap>,
}
// @@protoc_insertion_point(module)
