use std::vec;
use std::collections::HashMap;

pub trait ButtplugMessage {
    fn id(&self) -> u32;
}

pub trait ButtplugSystemMessage {
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct Ok {
    pub id: u32,
}

pub enum ErrorCode {
    ErrorUnknown = 0,
    ErrorInit,
    ErrorPing,
    ErrorMessage,
    ErrorDevice
}

#[derive(ButtplugMessage, ButtplugSystemMessage)]
pub struct Error {
    pub id: u32,
    pub error_code: ErrorCode,
    pub error_message: String,
}

pub struct MessageAttributes {
    pub feature_count: u32,
}

pub struct DeviceMessageInfo {
    pub device_index: u32,
    pub device_name: String,
    pub device_messages: Vec<String>,
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct DeviceList {
    pub id: u32,
    pub devices: Vec<DeviceMessageInfo>
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct DeviceAdded {
    pub id: u32,
    pub device_index: u32,
    pub device_name: String,
    pub device_messages: HashMap<String, MessageAttributes>
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct DeviceRemoved {
    pub id: u32,
    pub device_index: u32,
}

#[derive(Default, ButtplugMessage)]
pub struct StartScanning {
    pub id: u32,
}

#[derive(Default, ButtplugMessage)]
pub struct StopScanning {
    pub id: u32,
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct ScanningFinished {
    pub id: u32,
}

#[derive(Default, ButtplugMessage)]
pub struct RequestDeviceList {
    pub id: u32,
}

#[derive(Default, ButtplugMessage)]
pub struct RequestServerInfo {
    pub id: u32,
    pub client_name: String,
    pub message_version: u32,
}

#[derive(Default, ButtplugMessage, ButtplugSystemMessage)]
pub struct ServerInfo {
    pub id: u32,
    pub major_version: u32,
    pub minor_version: u32,
    pub build_version: u32,
    pub message_version: u32,
    pub max_ping_time: u32,
    pub server_name: String
}

pub enum ButtplugMessageUnion {
    Ok(Ok),
    Error(Error),
    DeviceList(DeviceList),
    DeviceAdded(DeviceAdded),
    DeviceRemoved(DeviceRemoved),
    StartScanning(StartScanning),
    StopScanning(StopScanning),
    ScanningFinished(ScanningFinished),
    RequestDeviceList(RequestDeviceList),
    RequestServerInfo(RequestServerInfo),
    ServerInfo(ServerInfo),
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
