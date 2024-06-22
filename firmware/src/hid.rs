/// HID Descriptor used in BLE keyboard, which might be different from USB HID device.
use usbd_hid::descriptor::generator_prelude::*;

// Source: <https://developer.nordicsemi.com/nRF5_SDK/nRF51_SDK_v4.x.x/doc/html/group___b_l_e___a_p_p_e_a_r_a_n_c_e_s.html#gac08ceb7b199eceefc4650399a3a7ff75>
pub const BLE_APPEARANCE_KEYBOARD: u16 = 0x03c1;
// Source: <https://the-sz.com/products/usbid/index.php?v=0x05AC&p=0x820A>
pub const APPLE_INC_VENDOR_ID: u16 = 0x05ac;
pub const APPLE_BLUETOOTH_HID_KEYBOARD_PRODUCT_ID: u16 = 0x820a;

/// Predefined report ids for composite BLE hid report. The report id of BLE should start from 0x01
/// Should be same with `#[gen_hid_descriptor]`
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReportType {
    Keyboard = 0x01,
    Mouse = 0x02,
    Media = 0x03,
    System = 0x04,
    Vial = 0x05,
}

/// KeyboardReport describes a report and its companion descriptor that can be
/// used to send keyboard button presses to a host and receive the status of the
/// keyboard LEDs.
#[gen_hid_descriptor(
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = KEYBOARD) = {
        (report_id = 0x01,) = {
            (usage_page = KEYBOARD, usage_min = 0xE0, usage_max = 0xE7) = {
                #[packed_bits 8] #[item_settings data,variable,absolute] modifier=input;
            };
            (usage_min = 0x00, usage_max = 0xFF) = {
                #[item_settings constant,variable,absolute] reserved=input;
            };
            (usage_page = LEDS, usage_min = 0x01, usage_max = 0x05) = {
                #[packed_bits 5] #[item_settings data,variable,absolute] leds=output;
            };
            (usage_page = KEYBOARD, usage_min = 0x00, usage_max = 0xDD) = {
                #[item_settings data,array,absolute] keycodes=input;
            };
        };
    },
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = MOUSE) = {
        (collection = PHYSICAL, usage = POINTER) = {
            (report_id = 0x02,) = {
                (usage_page = BUTTON, usage_min = BUTTON_1, usage_max = BUTTON_8) = {
                    #[packed_bits 8] #[item_settings data,variable,absolute] buttons=input;
                };
                (usage_page = GENERIC_DESKTOP,) = {
                    (usage = X,) = {
                        #[item_settings data,variable,relative] x=input;
                    };
                    (usage = Y,) = {
                        #[item_settings data,variable,relative] y=input;
                    };
                    (usage = WHEEL,) = {
                        #[item_settings data,variable,relative] wheel=input;
                    };
                };
                (usage_page = CONSUMER,) = {
                    (usage = AC_PAN,) = {
                        #[item_settings data,variable,relative] pan=input;
                    };
                };
            };
        };
    },
    (collection = APPLICATION, usage_page = CONSUMER, usage = CONSUMER_CONTROL) = {
        (report_id = 0x03,) = {
            (usage_page = CONSUMER, usage_min = 0x00, usage_max = 0x514) = {
            #[item_settings data,array,absolute,not_null] media_usage_id=input;
            }
        };
    },
    (collection = APPLICATION, usage_page = GENERIC_DESKTOP, usage = SYSTEM_CONTROL) = {
        (report_id = 0x04,) = {
            (usage_min = 0x81, usage_max = 0xB7, logical_min = 1) = {
                #[item_settings data,array,absolute,not_null] system_usage_id=input;
            };
        };
    },
    (collection = APPLICATION, usage_page = 0xFF60, usage = 0x61) = {
        (report_id = 0x05,) = {
            (usage = 0x62, logical_min = 0x0) = {
                #[item_settings data,variable,absolute] vial_input_data=input;
            };
            (usage = 0x63, logical_min = 0x0) = {
                #[item_settings data,variable,absolute] vial_output_data=output;
            };
        };
    }
)]
#[allow(dead_code)]
#[derive(Default)]
pub struct Report {
    pub modifier: u8,
    pub reserved: u8,
    pub leds: u8,
    pub keycodes: [u8; 6],
    pub buttons: u8,
    pub x: i8,
    pub y: i8,
    pub wheel: i8, // Scroll down (negative) or up (positive) this many units
    pub pan: i8,   // Scroll left (negative) or right (positive) this many units
    pub media_usage_id: u16,
    pub system_usage_id: u8,
    pub vial_input_data: [u8; 32],
    pub vial_output_data: [u8; 32],
}
