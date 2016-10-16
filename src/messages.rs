use std::vec::Vec;
use devices::DeviceInfo;

macro_rules! define_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        #[derive(Serialize, Deserialize, Clone)]
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
    }
}

macro_rules! define_msgs {
    (inner internal_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }

            pub fn as_message($($element: $ty),*) -> Message {
                return Message::Internal(InternalMessage::$name($name::new($($element),*)));
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

            pub fn as_message($($element: $ty),*) -> Message {
                return Message::Buttplug(ButtplugMessage::$name($name::new($($element),*)));
            }
        }
    };

    (inner device_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }

            pub fn as_message(device_id: u32, $($element: $ty),*) -> Message {
                return Message::Device(device_id, DeviceMessage::$name($name::new($($element),*)));
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

        // TODO We should be able to automate message enum creation via macros
        // but I'm too rusty on macro syntax right noq.

        // #[derive(Serialize, Deserialize)]
        // pub enum Message {
        //     $($msg_name($msg_name)),*
        // }
    };
}

define_msgs!(
    // TODO: This should be an internal message, just need to figure out how to
    // deal with serialization/deserialization
    internal_msg Shutdown ();
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

#[derive(Serialize, Deserialize, Clone)]
pub enum InternalMessage {
    Shutdown(Shutdown),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ButtplugMessage {
    DeviceListMessage(DeviceListMessage),
    RegisterClient(RegisterClient),
    ServerInfo(ServerInfo),
    Error(Error),
    Log(Log),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum DeviceMessage {
    ClaimDevice(ClaimDevice),
    ReleaseDevice(ReleaseDevice),
    LovenseRaw(LovenseRaw),
    SingleVibrateSpeed(SingleVibrateSpeed),
    ET312Raw(ET312Raw)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Message {
    Internal(InternalMessage),
    Buttplug(ButtplugMessage),
    Device(u32, DeviceMessage)
}

#[cfg(test)]
mod tests {
    use super::{ClaimDevice, ReleaseDevice, Message, ButtplugMessage, DeviceMessage};
    #[test]
    fn test_message_generation() {
        let msg = ClaimDevice::as_message(1);
        if let Message::Device(device_id, x) = msg {
            assert!(device_id == 1);
            if let DeviceMessage::ClaimDevice(m) = x {
                assert!(true);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }
}
