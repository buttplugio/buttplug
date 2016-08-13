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

macro_rules! define_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        #[derive(Default)]
        pub struct $a {
            pub msg_name: String,
            $(pub $element: $ty),*
        }
        impl ButtplugMessage for $a {
            fn name(&self) -> String {
                self.msg_name.clone()
            }
        }
    }
}

macro_rules! define_serializable_msg {
    ( $a: ident,
      $($element: ident: $ty: ty),*) =>
    {
        #[derive(Serialize, Deserialize, Default)]
        pub struct $a {
            pub msg_name: String,
            $(pub $element: $ty),*
        }
        impl ButtplugMessage for $a {
            fn name(&self) -> String {
                self.msg_name.clone()
            }
        }
    }
}

macro_rules! define_msgs {
    (inner internal_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    msg_name: stringify!($name).to_string(),
                    $($element: $element),*
                }
            }
        }
    };

    (inner base_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_serializable_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    msg_name: stringify!($name).to_string(),
                    $($element: $element),*
                }
            }
        }
    };

    (inner base_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_serializable_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    msg_name: stringify!($name).to_string(),
                    $($element: $element),*
                }
            }
        }
    };

    (inner device_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_serializable_msg!($name, device_id: u32 $(,$element: $ty),*);
        impl $name {
            pub fn new(device_id: u32, $($element: $ty),*) -> $name {
                $name {
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
    //base_msg DeviceListMessage (devices: Vec<DeviceInfo>);
    internal_msg Shutdown ();
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
        assert!(msg.name() == "ClaimDevice");
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
