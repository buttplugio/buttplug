use std::vec::Vec;
use devices::DeviceInfo;

pub trait ButtplugMessage {
    fn device_id(&self) -> Option<u32>;
}

macro_rules! define_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        #[derive(Serialize, Deserialize)]
        pub struct $a {
            $(pub $element: $ty),*
        }
    }
}

macro_rules! define_non_device_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        define_msg!($a, $($element: $ty),*);
        impl ButtplugMessage for $a {
            fn device_id(&self) -> Option<u32> {
                None
            }
        }
    }
}

macro_rules! define_msgs {
    (inner base_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }
        }
    };

    (inner base_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }
        }
    };

    (inner device_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_msg!($name, device_id: u32 $(,$element: $ty),*);
        impl $name {
            pub fn new(device_id: u32, $($element: $ty),*) -> $name {
                $name {
                    device_id: device_id,
                    $($element: $element),*
                }
            }
        }
        impl ButtplugMessage for $name {
            fn device_id(&self) -> Option<u32> {
                Some(self.device_id)
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

        #[derive(Serialize, Deserialize)]
        pub enum Message {
            $($msg_name($msg_name)),*
        }
    };
}

define_msgs!(
    // TODO: This should be an internal message, just need to figure out how to
    // deal with serialization/deserialization
    base_msg Shutdown ();
    base_msg DeviceListMessage (devices: Vec<DeviceInfo>);
    base_msg RegisterClient ();
    base_msg ServerInfo ();
    base_msg Error(error_str: String);
    base_msg Log(log_str: String);
    device_msg ClaimDevice ();
    device_msg ReleaseDevice ();
    device_msg LovenseRaw (speed:u32);
    device_msg SingleVibrateSpeed(speed:u32);
    device_msg ET312Raw(msg: Vec<u8>)
);

#[cfg(test)]
mod tests {
    use super::{ClaimDevice, ReleaseDevice, Message, ButtplugMessage};
    #[test]
    fn test_message_generation() {
        let msg = ClaimDevice::new(1);
        assert!(msg.device_id == 1);
    }

    #[test]
    fn test_message_enum() {
        let enum_msg = Message::ClaimDevice(ClaimDevice::new(1));
        match enum_msg {
            Message::ClaimDevice(msg) => assert!(true),
            _ => assert!(false)
        }
    }
}
