use std::vec::Vec;

trait ButtplugMessage {
    fn name(&self) -> String;
}

trait ButtplugDeviceMessage {
    fn device_id(&self) -> u32;
}

pub struct DeviceInfo {
    pub device_name: String,
    pub device_id: u32
}

macro_rules! define_msg_base {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        pub struct $a {
            pub msg_name: String,
            $(pub $element: $ty),*
        }
        impl ButtplugMessage for $a {
            fn name(&self) -> String {
                return self.msg_name.clone();
            }
        }
    }
}

macro_rules! define_msgs {
    (inner base_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_msg_base!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                return $name {
                    msg_name: stringify!($name).to_string(),
                    $($element: $element),*
                }
            }
        }
    };

    (inner device_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_msg_base!($name, device_id: u32 $(,$element: $ty),*);
        impl $name {
            pub fn new(device_id: u32, $($element: $ty),*) -> $name {
                return $name {
                    msg_name: stringify!($name).to_string(),
                    device_id: device_id,
                    $($element: $element),*
                }
            }
        }
    };

    (
        $(
            $msg_type: ident $msg_name: ident ($($element: ident: $ty: ty),*)
        );*
    ) =>
    {
        $(define_msgs!(inner $msg_type $msg_name $($element: $ty),*);)*

        pub enum Message {
            $($msg_name($msg_name)),*
        }
    };
}

define_msgs!(
    base_msg DeviceListMessage (devices: Vec<DeviceInfo>);
    device_msg ClaimDeviceMessage ();
    device_msg ReleaseDeviceMessage ();
    device_msg LovenseRawMessage (speed:u32);
    device_msg SingleVibrateSpeedMessage(speed:u32);
    device_msg ET312RawMessage(msg: Vec<u8>)
);

#[cfg(test)]
mod tests {
    use super::{ClaimDeviceMessage, ReleaseDeviceMessage, ButtplugMessage, Message};
    #[test]
    fn test_message_generation() {
        let msg = ClaimDeviceMessage::new(1);
        assert!(msg.name() == "ClaimDeviceMessage");
        assert!(msg.device_id == 1);
    }

    #[test]
    fn test_message_enum() {
        let enum_msg = Message::ClaimDeviceMessage(ClaimDeviceMessage::new(1));
        match enum_msg {
            Message::ClaimDeviceMessage(msg) => assert!(true),
            _ => assert!(false)
        }
    }
}
