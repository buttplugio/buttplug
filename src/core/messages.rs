use std::collections::HashMap;
use super::errors::*;

pub trait ButtplugMessage: Send + Sync + Clone {
    fn get_id(&self) -> u32;
    fn set_id(&mut self, id: u32);
    fn as_union(self) -> ButtplugMessageUnion;
}

#[derive(Debug, PartialEq, Default, ButtplugMessage, Clone)]
pub struct Ok {
    id: u32,
}

impl Ok {
    pub fn new() -> Ok {
        Ok {
            id: 0
        }
    }
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    ErrorUnknown = 0,
    ErrorInit,
    ErrorPing,
    ErrorMessage,
    ErrorDevice
}

#[derive(Debug, ButtplugMessage, Clone)]
pub struct Error {
    id: u32,
    pub error_code: ErrorCode,
    pub error_message: String,
}

impl Error {
    pub fn new(error_code: ErrorCode, error_message: &str) -> Error {
        Error {
            id: 0,
            error_code: error_code,
            error_message: error_message.to_string()
        }
    }
}

impl From<ButtplugError> for Error {
    fn from(error: ButtplugError) -> Self {
        let code = match error {
            ButtplugError::ButtplugDeviceError(_) => ErrorCode::ErrorDevice,
            ButtplugError::ButtplugMessageError(_) => ErrorCode::ErrorMessage,
            ButtplugError::ButtplugPingError(_) => ErrorCode::ErrorPing,
            ButtplugError::ButtplugInitError(_) => ErrorCode::ErrorInit,
            ButtplugError::ButtplugUnknownError(_) => ErrorCode::ErrorUnknown,
        };
        // Gross but was having problems with naming collisions on the error trait
        let msg = match error {
            ButtplugError::ButtplugDeviceError(_s) => _s.message,
            ButtplugError::ButtplugMessageError(_s) => _s.message,
            ButtplugError::ButtplugPingError(_s) => _s.message,
            ButtplugError::ButtplugInitError(_s) => _s.message,
            ButtplugError::ButtplugUnknownError(_s) => _s.message,
        };
        Error::new(code, &msg)
    }
}

#[derive(Clone, Debug)]
pub struct MessageAttributes {
    pub feature_count: u32,
}

#[derive(Clone, Debug)]
pub struct DeviceMessageInfo {
    pub device_index: u32,
    pub device_name: String,
    pub device_messages: Vec<String>,
}

#[derive(Default, ButtplugMessage, Clone, Debug)]
pub struct DeviceList {
    id: u32,
    pub devices: Vec<DeviceMessageInfo>
}

#[derive(Default, ButtplugMessage, Clone, Debug)]
pub struct DeviceAdded {
    id: u32,
    pub device_index: u32,
    pub device_name: String,
    pub device_messages: HashMap<String, MessageAttributes>
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct DeviceRemoved {
    id: u32,
    pub device_index: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct StartScanning {
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct StopScanning {
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct ScanningFinished {
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct RequestDeviceList {
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone)]
pub struct RequestServerInfo {
    id: u32,
    pub client_name: String,
    pub message_version: u32,
}

impl RequestServerInfo {
    pub fn new(client_name: &str, message_version: u32) -> RequestServerInfo {
        RequestServerInfo {
            id: 0,
            client_name: client_name.to_string(),
            message_version: message_version
        }
    }
}

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone)]
pub struct ServerInfo {
    id: u32,
    pub major_version: u32,
    pub minor_version: u32,
    pub build_version: u32,
    pub message_version: u32,
    pub max_ping_time: u32,
    pub server_name: String
}

impl ServerInfo {
    pub fn new(server_name: &str, message_version: u32, max_ping_time: u32) -> ServerInfo {
        ServerInfo {
            id: 0,
            major_version: 0,
            minor_version: 0,
            build_version: 0,
            message_version: message_version,
            max_ping_time: max_ping_time,
            server_name: server_name.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
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

impl ButtplugMessage for ButtplugMessageUnion {
    fn get_id(&self) -> u32 {
        match self {
            ButtplugMessageUnion::Ok (ref _msg) => return _msg.id,
            ButtplugMessageUnion::Error (ref _msg) => return _msg.id,
            ButtplugMessageUnion::DeviceList (ref _msg) => return _msg.id,
            ButtplugMessageUnion::DeviceAdded (ref _msg) => return _msg.id,
            ButtplugMessageUnion::DeviceRemoved (ref _msg) => return _msg.id,
            ButtplugMessageUnion::StartScanning (ref _msg) => return _msg.id,
            ButtplugMessageUnion::StopScanning (ref _msg) => return _msg.id,
            ButtplugMessageUnion::ScanningFinished (ref _msg) => return _msg.id,
            ButtplugMessageUnion::RequestDeviceList (ref _msg) => return _msg.id,
            ButtplugMessageUnion::RequestServerInfo (ref _msg) => return _msg.id,
            ButtplugMessageUnion::ServerInfo (ref _msg) => return _msg.id,
        }
    }

    fn set_id(&mut self, id: u32) {
        match self {
            ButtplugMessageUnion::Ok (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::Error (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::DeviceList (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::DeviceAdded (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::DeviceRemoved (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::StartScanning (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::StopScanning (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::ScanningFinished (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::RequestDeviceList (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::RequestServerInfo (ref mut _msg) => _msg.set_id(id),
            ButtplugMessageUnion::ServerInfo (ref mut _msg) => _msg.set_id(id),
        }
    }

    fn as_union(self) -> ButtplugMessageUnion {
        panic!("as_union shouldn't be called on union.");
    }
}
