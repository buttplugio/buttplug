// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Structs representing low level [Buttplug
//! Protocol](https://buttplug-spec.docs.buttplug.io) messages

use super::errors::*;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;

/// Base trait for all Buttplug Protocol Message Structs. Handles management of
/// message ids, as well as implementing conveinence functions for converting
/// between message structs and [ButtplugMessageUnion] enums, serialization, etc...
pub trait ButtplugMessage: Send + Sync + Clone + Serialize + Deserialize<'static> {
    /// Returns the id number of the message
    fn get_id(&self) -> u32;
    /// Sets the id number of the message
    fn set_id(&mut self, id: u32);
    /// Returns the message as a [ButtplugMessageUnion] enum.
    fn as_union(self) -> ButtplugMessageUnion;
    /// Returns the message as a string in Buttplug JSON Protocol format.
    fn as_protocol_json(&self) -> String {
        "[".to_owned() + &serde_json::to_string(&self).unwrap() + "]"
    }
}

/// Represents the Buttplug Protocol Ok message, as documented in the [Buttplug
/// Protocol Spec](https://buttplug-spec.docs.buttplug.io/status.html#ok).
#[derive(Debug, PartialEq, Default, ButtplugMessage, Clone, Serialize, Deserialize)]
pub struct Ok {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[serde(rename = "Id")]
    id: u32,
}

impl Ok {
    /// Creates a new Ok message with the given Id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

/// Error codes pertaining to error classes that can be represented in the
/// Buttplug [Error] message.
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, PartialEq)]
#[repr(u8)]
pub enum ErrorCode {
    ErrorUnknown = 0,
    ErrorHandshake,
    ErrorPing,
    ErrorMessage,
    ErrorDevice,
}

/// Represents the Buttplug Protocol Error message, as documented in the [Buttplug
/// Protocol Spec](https://buttplug-spec.docs.buttplug.io/status.html#error).
#[derive(Debug, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct Error {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[serde(rename = "Id")]
    id: u32,
    /// Specifies the class of the error.
    #[serde(rename = "ErrorCode")]
    pub error_code: ErrorCode,
    /// Description of the error.
    #[serde(rename = "ErrorMessage")]
    pub error_message: String,
}

impl Error {
    /// Creates a new error object.
    pub fn new(error_code: ErrorCode, error_message: &str) -> Self {
        Self {
            id: 0,
            error_code,
            error_message: error_message.to_string(),
        }
    }
}

impl From<ButtplugError> for Error {
    /// Converts a [super::errors::ButtplugError] object into a Buttplug Protocol
    /// [Error] message.
    fn from(error: ButtplugError) -> Self {
        let code = match error {
            ButtplugError::ButtplugDeviceError(_) => ErrorCode::ErrorDevice,
            ButtplugError::ButtplugMessageError(_) => ErrorCode::ErrorMessage,
            ButtplugError::ButtplugPingError(_) => ErrorCode::ErrorPing,
            ButtplugError::ButtplugHandshakeError(_) => ErrorCode::ErrorHandshake,
            ButtplugError::ButtplugUnknownError(_) => ErrorCode::ErrorUnknown,
        };
        // Gross but was having problems with naming collisions on the error trait
        let msg = match error {
            ButtplugError::ButtplugDeviceError(_s) => _s.message,
            ButtplugError::ButtplugMessageError(_s) => _s.message,
            ButtplugError::ButtplugPingError(_s) => _s.message,
            ButtplugError::ButtplugHandshakeError(_s) => _s.message,
            ButtplugError::ButtplugUnknownError(_s) => _s.message,
        };
        Error::new(code, &msg)
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct Ping {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[serde(rename = "Id")]
    id: u32,
}

impl Ping {
    /// Creates a new Ping message with the given Id.
    pub fn new(id: u32) -> Self {
        Self { id }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct Test {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[serde(rename = "Id")]
    id: u32,
    /// Test string, which will be echo'd back to client when sent to server.
    #[serde(rename = "TestString")]
    test_string: String,
}

impl Test {
    /// Creates a new Ping message with the given Id.
    pub fn new(test: &str) -> Self {
        Self {
            id:1,
            test_string: test.to_owned()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MessageAttributes {
    #[serde(rename = "FeatureCount")]
    pub feature_count: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeviceMessageInfo {
    #[serde(rename = "DeviceIndex")]
    pub device_index: u32,
    #[serde(rename = "DeviceName")]
    pub device_name: String,
    #[serde(rename = "DeviceMessages")]
    pub device_messages: HashMap<String, MessageAttributes>,
}

#[derive(Default, ButtplugMessage, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeviceList {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "Devices")]
    pub devices: Vec<DeviceMessageInfo>,
}

#[derive(Default, ButtplugMessage, Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DeviceAdded {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "DeviceIndex")]
    pub device_index: u32,
    #[serde(rename = "DeviceName")]
    pub device_name: String,
    #[serde(rename = "DeviceMessages")]
    pub device_messages: HashMap<String, MessageAttributes>,
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceRemoved {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "DeviceIndex")]
    pub device_index: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartScanning {
    #[serde(rename = "Id")]
    id: u32,
}

impl StartScanning {
    pub fn new() -> Self {
        Self { id: 1 }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct StopScanning {
    #[serde(rename = "Id")]
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScanningFinished {
    #[serde(rename = "Id")]
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestDeviceList {
    #[serde(rename = "Id")]
    id: u32,
}

#[derive(Debug, Default, ButtplugMessage, Clone, Serialize, Deserialize, PartialEq)]
pub struct RequestServerInfo {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "ClientName")]
    pub client_name: String,
    #[serde(rename = "MessageVersion")]
    pub message_version: u32,
}

impl RequestServerInfo {
    pub fn new(client_name: &str, message_version: u32) -> Self {
        Self {
            id: 1,
            client_name: client_name.to_string(),
            message_version,
        }
    }
}

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "MajorVersion")]
    pub major_version: u32,
    #[serde(rename = "MinorVersion")]
    pub minor_version: u32,
    #[serde(rename = "BuildVersion")]
    pub build_version: u32,
    #[serde(rename = "MessageVersion")]
    pub message_version: u32,
    #[serde(rename = "MaxPingTime")]
    pub max_ping_time: u32,
    #[serde(rename = "ServerName")]
    pub server_name: String,
}

impl ServerInfo {
    pub fn new(server_name: &str, message_version: u32, max_ping_time: u32) -> Self {
        Self {
            id: 0,
            major_version: 0,
            minor_version: 0,
            build_version: 0,
            message_version,
            max_ping_time,
            server_name: server_name.to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Off = 0,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "LogLevel")]
    pub log_level: LogLevel,
}

impl RequestLog {
    pub fn new(log_level: LogLevel) -> Self {
        Self {
            id: 1,
            log_level,
        }
    }
}

#[derive(Debug, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct Log {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "LogLevel")]
    pub log_level: LogLevel,
    #[serde(rename = "LogMessage")]
    pub log_message: String,
}

impl Log {
    pub fn new(log_level: LogLevel, log_message: String) -> Self {
        Self {
            id: 0,
            log_level,
            log_message,
        }
    }
}

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct StopDeviceCmd {
    #[serde(rename = "Id")]
    pub id: u32,
    #[serde(rename = "DeviceIndex")]
    pub device_index: u32,
}

impl StopDeviceCmd {
    pub fn new(device_index: u32) -> Self {
        Self {
            id: 1,
            device_index,
        }
    }
}

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct StopAllDevices {
    #[serde(rename = "Id")]
    pub id: u32,
}

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
pub struct VibrateSubcommand {
    #[serde(rename = "Index")]
    pub index: u32,
    #[serde(rename = "Speed")]
    pub speed: f64,
}

impl VibrateSubcommand {
    pub fn new(index: u32, speed: f64) -> Self {
        Self { index, speed }
    }
}

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone, Serialize, Deserialize)]
pub struct VibrateCmd {
    #[serde(rename = "Id")]
    pub id: u32,
    #[serde(rename = "DeviceIndex")]
    pub device_index: u32,
    #[serde(rename = "Speeds")]
    pub speeds: Vec<VibrateSubcommand>,
}

impl VibrateCmd {
    pub fn new(device_index: u32, speeds: Vec<VibrateSubcommand>) -> Self {
        Self {
            id: 1,
            device_index,
            speeds,
        }
    }
}



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ButtplugMessageUnion {
    Ok(Ok),
    Error(Error),
    Ping(Ping),
    Test(Test),
    RequestLog(RequestLog),
    Log(Log),
    RequestServerInfo(RequestServerInfo),
    ServerInfo(ServerInfo),
    DeviceList(DeviceList),
    DeviceAdded(DeviceAdded),
    DeviceRemoved(DeviceRemoved),
    StartScanning(StartScanning),
    StopScanning(StopScanning),
    ScanningFinished(ScanningFinished),
    RequestDeviceList(RequestDeviceList),
    VibrateCmd(VibrateCmd),
    StopDeviceCmd(StopDeviceCmd),
    StopAllDevices(StopAllDevices),
}

impl ButtplugMessage for ButtplugMessageUnion {
    fn get_id(&self) -> u32 {
        match self {
            ButtplugMessageUnion::Ok(ref msg) => msg.id,
            ButtplugMessageUnion::Error(ref msg) => msg.id,
            ButtplugMessageUnion::Log(ref msg) => msg.id,
            ButtplugMessageUnion::RequestLog(ref msg) => msg.id,
            ButtplugMessageUnion::Ping(ref msg) => msg.id,
            ButtplugMessageUnion::Test(ref msg) => msg.id,
            ButtplugMessageUnion::RequestServerInfo(ref msg) => msg.id,
            ButtplugMessageUnion::ServerInfo(ref msg) => msg.id,
            ButtplugMessageUnion::DeviceList(ref msg) => msg.id,
            ButtplugMessageUnion::DeviceAdded(ref msg) => msg.id,
            ButtplugMessageUnion::DeviceRemoved(ref msg) => msg.id,
            ButtplugMessageUnion::StartScanning(ref msg) => msg.id,
            ButtplugMessageUnion::StopScanning(ref msg) => msg.id,
            ButtplugMessageUnion::ScanningFinished(ref msg) => msg.id,
            ButtplugMessageUnion::RequestDeviceList(ref msg) => msg.id,
            ButtplugMessageUnion::VibrateCmd(ref msg) => msg.id,
            ButtplugMessageUnion::StopDeviceCmd(ref msg) => msg.id,
            ButtplugMessageUnion::StopAllDevices(ref msg) => msg.id,
        }
    }

    fn set_id(&mut self, id: u32) {
        match self {
            ButtplugMessageUnion::Ok(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::Error(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::Log(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::RequestLog(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::Ping(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::Test(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::RequestServerInfo(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::ServerInfo(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::DeviceList(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::DeviceAdded(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::DeviceRemoved(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::StartScanning(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::StopScanning(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::ScanningFinished(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::RequestDeviceList(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::VibrateCmd(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::StopDeviceCmd(ref mut msg) => msg.set_id(id),
            ButtplugMessageUnion::StopAllDevices(ref mut msg) => msg.set_id(id),
        }
    }

    fn as_union(self) -> ButtplugMessageUnion {
        panic!("as_union shouldn't be called on union.");
    }
}

#[cfg(test)]
mod test {
    use super::{ButtplugMessageUnion, Error, ErrorCode, Ok};

    const OK_STR: &str = "{\"Ok\":{\"Id\":0}}";
    const ERROR_STR: &str =
        "{\"Error\":{\"Id\":0,\"ErrorCode\":1,\"ErrorMessage\":\"Test Error\"}}";

    #[test]
    fn test_ok_serialize() {
        let ok = ButtplugMessageUnion::Ok(Ok::new(0));
        let js = serde_json::to_string(&ok).unwrap();
        assert_eq!(OK_STR, js);
    }

    #[test]
    fn test_ok_deserialize() {
        let union: ButtplugMessageUnion = serde_json::from_str(&OK_STR).unwrap();
        assert_eq!(ButtplugMessageUnion::Ok(Ok::new(0)), union);
    }

    #[test]
    fn test_error_serialize() {
        let error =
            ButtplugMessageUnion::Error(Error::new(ErrorCode::ErrorHandshake, "Test Error"));
        let js = serde_json::to_string(&error).unwrap();
        assert_eq!(ERROR_STR, js);
    }

    #[test]
    fn test_error_deserialize() {
        let union: ButtplugMessageUnion = serde_json::from_str(&ERROR_STR).unwrap();
        assert_eq!(
            ButtplugMessageUnion::Error(Error::new(ErrorCode::ErrorHandshake, "Test Error")),
            union
        );
    }
}
