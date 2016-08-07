use std::vec::Vec;
use std::option::Option;

trait ButtplugMessage {
    fn name(&self) -> String;
}

macro_rules! define_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        pub struct $a {
            pub msg_name: String,
            $(pub $element: $ty),*
        }
        impl $a {
            pub fn new($($element: $ty),*) -> $a {
                return $a {
                    msg_name: stringify!($a).to_string(),
                    $($element: $element),*
                }
            }
        }
        impl ButtplugMessage for $a {
            fn name(&self) -> String {
                return self.msg_name.clone();
            }
        }
    }
}

pub struct DeviceInfo {
    pub device_name: String,
    pub device_id: u32
}

define_msg!(DeviceListMessage, devices: Vec<DeviceInfo>);
define_msg!(ClaimDeviceMessage, device_id: u32);
define_msg!(ReleaseDeviceMessage, device_id: u32);
define_msg!(LovenseRawMessage, device_id: u32, command: String);
define_msg!(SingleVibrateSpeedMessage, device_id: u32, speed: u8);
define_msg!(ET312RawMessage, device_id: u32, command: Vec<u8>);

pub enum Messages {
    DeviceListMessage,
    ClaimDeviceMessage,
    ReleaseDeviceMessage,
    LovenseRawMessage,
    SingleVibrateSpeedMessage,
    ET312RawMessage
}

#[cfg(test)]
mod tests {
    use super::{ClaimDeviceMessage, ButtplugMessage};
    #[test]
    fn test_message_generation() {
        let msg = ClaimDeviceMessage::new(0);
        println!("{}", msg.name());
        assert!(msg.name() == "ClaimDeviceMessage");
        assert!(msg.device_id == 0);
    }
}
