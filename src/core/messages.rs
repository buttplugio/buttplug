use std::vec;
use std::collections::HashMap;

trait ButtplugMessage {
    fn id(&self) -> u32;
}

#[derive(Default, ButtplugMessage)]
struct Ok {
    id: u32,
}

#[derive(ButtplugMessage)]
struct Error {
    id: u32,
    error_code: u32,
    error_message: str,
}

struct MessageAttributes {
    feature_count: u32,
}

struct DeviceMessageInfo {
    device_index: u32,
    device_name: String,
    device_messages: Vec<String>,
}

#[derive(ButtplugMessage)]
struct DeviceList {
    id: u32,
    devices: Vec<DeviceMessageInfo>
}

#[derive(ButtplugMessage)]
struct DeviceAdded {
    id: u32,
    device_index: u32,
    device_name: String,
    device_messages: HashMap<String, MessageAttributes>
}

#[derive(ButtplugMessage)]
struct StartScanning {
    id: u32,
}

#[derive(ButtplugMessage)]
struct StopScanning {
    id: u32,
}

#[derive(ButtplugMessage)]
struct ScanningFinished {
    id: u32,
}

#[derive(ButtplugMessage)]
struct RequestDeviceList {
    id: u32,
}

#[derive(ButtplugMessage)]
struct RequestServerInfo {
    id: u32,
    client_name: String,
    message_version: u32,
}

#[derive(ButtplugMessage)]
struct ServerInfo {
    id: u32,
    major_version: u32,
    minor_version: u32,
    build_version: u32,
    message_version: u32,
    max_ping_time: u32,
    server_name: String
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
