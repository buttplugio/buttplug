use std::vec::Vec;

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
                return Message::Internal(Internal::$name($name::new($($element),*)));
            }
        }
    };

    (inner host_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }

            pub fn as_message($($element: $ty),*) -> Message {
                return Message::Host(Host::$name($name::new($($element),*)));
            }
        }
    };

    (inner client_msg $name: ident $($element: ident: $ty: ty),*) =>
    {
        define_non_device_msg!($name, $($element: $ty),*);
        impl $name {
            pub fn new($($element: $ty),*) -> $name {
                $name {
                    $($element: $element),*
                }
            }

            pub fn as_message($($element: $ty),*) -> Message {
                return Message::Client(Client::$name($name::new($($element),*)));
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
                return Message::Device(device_id, Device::$name($name::new($($element),*)));
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
        // but I'm too rusty on macro syntax right now.

        // #[derive(Serialize, Deserialize)]
        // pub enum Message {
        //     $($msg_name($msg_name)),*
        // }
    };
}

define_msgs!(
    internal_msg Shutdown ();
    client_msg RequestDeviceList();
    host_msg DeviceList(devices: Vec<(u32, String)>);
    client_msg RegisterClient(client_info: String);
    host_msg ClientRegistered();
    client_msg RequestServerInfo();
    host_msg ServerInfo(version: String);
    host_msg Error(error_str: String);
    host_msg Log(log_str: String);
    host_msg Ping();
    host_msg DeviceClaimed(id: u32, token: u32);
    host_msg Ok();
    client_msg Pong();
    device_msg ClaimDevice ();
    device_msg ReleaseDevice ();
    device_msg LovenseRaw (speed:u32);
    device_msg SingleVibrateSpeed(speed:u32);
    device_msg ET312Raw(msg: Vec<u8>)
);

#[derive(Serialize, Deserialize, Clone)]
pub enum Internal {
    Shutdown(Shutdown),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Client {
    RequestDeviceList(RequestDeviceList),
    RegisterClient(RegisterClient),
    RequestServerInfo(RequestServerInfo),
    Pong(Pong)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Host {
    DeviceList(DeviceList),
    DeviceClaimed(DeviceClaimed),
    ClientRegistered(ClientRegistered),
    ServerInfo(ServerInfo),
    Ok(Ok),
    Error(Error),
    Log(Log),
    Ping(Ping)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Device {
    ClaimDevice(ClaimDevice),
    ReleaseDevice(ReleaseDevice),
    LovenseRaw(LovenseRaw),
    SingleVibrateSpeed(SingleVibrateSpeed),
    ET312Raw(ET312Raw)
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Message {
    Internal(Internal),
    Client(Client),
    Host(Host),
    Device(u32, Device)
}

pub struct IncomingMessage {
    pub msg: Message,
    pub callback: Box<Fn(Message) + Send>
}

#[cfg(test)]
mod tests {
    use super::{ClaimDevice, ReleaseDevice, Message, Device};
    #[test]
    fn test_message_generation() {
        let msg = ClaimDevice::as_message(1);
        if let Message::Device(device_id, x) = msg {
            assert!(device_id == 1);
            if let Device::ClaimDevice(m) = x {
                assert!(true);
            } else {
                assert!(false);
            }
        } else {
            assert!(false);
        }
    }
}
