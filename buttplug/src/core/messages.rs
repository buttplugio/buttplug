// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Structs representing low level [Buttplug
//! Protocol](https://buttplug-spec.docs.buttplug.io) messages

use super::errors::*;
use crate::device::Endpoint;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize_json")]
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{
    collections::HashMap,
    convert::{From, TryFrom},
};

/// Base trait for all Buttplug Protocol Message Structs. Handles management of
/// message ids, as well as implementing conveinence functions for converting
/// between message structs and [ButtplugMessageUnion] enums, serialization, etc...
pub trait ButtplugMessage: Send + Sync + Clone {
    /// Returns the id number of the message
    fn get_id(&self) -> u32;
    /// Sets the id number of the message.
    fn set_id(&mut self, id: u32);
    /// Returns the message as a string in Buttplug JSON Protocol format.
    #[cfg(feature = "serialize_json")]
    fn as_protocol_json(self) -> String
    where
        Self: ButtplugMessage + Serialize + Deserialize<'static>,
    {
        serde_json::to_string(&[&self]).unwrap()
    }
}

pub trait ButtplugDeviceMessage: ButtplugMessage {
    fn get_device_index(&self) -> u32;
    fn set_device_index(&mut self, id: u32);
}

/// Represents the Buttplug Protocol Ok message, as documented in the [Buttplug
/// Protocol Spec](https://buttplug-spec.docs.buttplug.io/status.html#ok).
#[derive(Debug, PartialEq, Default, ButtplugMessage, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Ok {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
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
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize_repr, Deserialize_repr))]
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
#[derive(Debug, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Error {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    /// Specifies the class of the error.
    #[cfg_attr(feature = "serialize_json", serde(rename = "ErrorCode"))]
    pub error_code: ErrorCode,
    /// Description of the error.
    #[cfg_attr(feature = "serialize_json", serde(rename = "ErrorMessage"))]
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

#[derive(Debug, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Ping {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
}

impl Default for Ping {
    /// Creates a new Ping message with the given Id.
    fn default() -> Self {
        Self { id: 1 }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Test {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    /// Test string, which will be echo'd back to client when sent to server.
    #[cfg_attr(feature = "serialize_json", serde(rename = "TestString"))]
    pub test_string: String,
}

impl Test {
    /// Creates a new Ping message with the given Id.
    pub fn new(test: &str) -> Self {
        Self {
            id: 1,
            test_string: test.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct MessageAttributes {
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "FeatureCount"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub feature_count: Option<u32>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "StepCount"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub step_count: Option<Vec<u32>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "Endpoints"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub endpoints: Option<Vec<Endpoint>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "MaxDuration"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub max_duration: Option<Vec<u32>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "Patterns"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub patterns: Option<Vec<Vec<String>>>,
    #[cfg_attr(
        feature = "serialize_json",
        serde(rename = "ActuatorType"),
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub actuator_type: Option<Vec<String>>,
    // Never serialize this, its for internal use only
    #[cfg_attr(feature = "serialize_json", serde(rename = "FeatureOrder"))]
    pub feature_order: Option<Vec<u32>>,
}

pub type MessageAttributesMap = HashMap<String, MessageAttributes>;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfo {
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
    pub device_name: String,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
    pub device_messages: MessageAttributesMap,
}

impl From<&DeviceAdded> for DeviceMessageInfo {
    fn from(device_added: &DeviceAdded) -> Self {
        Self {
            device_index: device_added.device_index,
            device_name: device_added.device_name.clone(),
            device_messages: device_added.device_messages.clone(),
        }
    }
}

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceList {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
    pub devices: Vec<DeviceMessageInfo>,
}

impl DeviceList {
    pub fn new(devices: Vec<DeviceMessageInfo>) -> Self {
        Self { id: 0, devices }
    }
}

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceAdded {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
    pub device_name: String,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
    pub device_messages: MessageAttributesMap,
}

impl DeviceAdded {
    pub fn new(
        device_index: u32,
        device_name: &String,
        device_messages: &MessageAttributesMap,
    ) -> Self {
        Self {
            id: 0,
            device_index,
            device_name: device_name.to_string(),
            device_messages: device_messages.clone(),
        }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceRemoved {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
}

#[derive(Debug, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct StartScanning {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
}

impl Default for StartScanning {
    fn default() -> Self {
        Self { id: 1 }
    }
}

#[derive(Debug, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct StopScanning {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
}

impl Default for StopScanning {
    fn default() -> Self {
        Self { id: 1 }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct ScanningFinished {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
}

#[derive(Debug, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RequestDeviceList {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
}

impl Default for RequestDeviceList {
    fn default() -> Self {
        Self { id: 1 }
    }
}

#[derive(Debug, Default, ButtplugMessage, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RequestServerInfo {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "ClientName"))]
    pub client_name: String,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MessageVersion"))]
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

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct ServerInfo {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MajorVersion"))]
    pub major_version: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MinorVersion"))]
    pub minor_version: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "BuildVersion"))]
    pub build_version: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MessageVersion"))]
    pub message_version: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MaxPingTime"))]
    pub max_ping_time: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "ServerName"))]
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

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum LogLevel {
    Off = 0,
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RequestLog {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "LogLevel"))]
    pub log_level: LogLevel,
}

impl RequestLog {
    pub fn new(log_level: LogLevel) -> Self {
        Self { id: 1, log_level }
    }
}

#[derive(Debug, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Log {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "LogLevel"))]
    pub log_level: LogLevel,
    #[cfg_attr(feature = "serialize_json", serde(rename = "LogMessage"))]
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

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct StopDeviceCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
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

#[derive(Debug, Default, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct StopAllDevices {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
}

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VibrateSubcommand {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Index"))]
    pub index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: f64,
}

impl VibrateSubcommand {
    pub fn new(index: u32, speed: f64) -> Self {
        Self { index, speed }
    }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VibrateCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speeds"))]
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

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VectorSubcommand {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Index"))]
    pub index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Duration"))]
    pub duration: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Position"))]
    pub position: f64,
}

impl VectorSubcommand {
    pub fn new(index: u32, duration: u32, position: f64) -> Self {
        Self {
            index,
            duration,
            position,
        }
    }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct LinearCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Vectors"))]
    pub vectors: Vec<VectorSubcommand>,
}

impl LinearCmd {
    pub fn new(device_index: u32, vectors: Vec<VectorSubcommand>) -> Self {
        Self {
            id: 1,
            device_index,
            vectors,
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RotationSubcommand {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Index"))]
    pub index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: f64,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Clockwise"))]
    pub clockwise: bool,
}

impl RotationSubcommand {
    pub fn new(index: u32, speed: f64, clockwise: bool) -> Self {
        Self {
            index,
            speed,
            clockwise,
        }
    }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RotateCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Rotations"))]
    pub rotations: Vec<RotationSubcommand>,
}

impl RotateCmd {
    pub fn new(device_index: u32, rotations: Vec<RotationSubcommand>) -> Self {
        Self {
            id: 1,
            device_index,
            rotations,
        }
    }
}

#[derive(Debug, Default, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct FleshlightLaunchFW12Cmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Position"))]
    pub position: u8,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: u8,
}

impl FleshlightLaunchFW12Cmd {
    pub fn new(device_index: u32, position: u8, speed: u8) -> Self {
        Self {
            id: 1,
            device_index,
            position,
            speed,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct LovenseCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Command"))]
    pub command: String,
}

impl LovenseCmd {
    pub fn new(device_index: u32, command: &str) -> Self {
        Self {
            id: 1,
            device_index,
            command: command.to_owned(),
        }
    }
}

// Dear god this needs to be deprecated
#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct KiirooCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Command"))]
    pub command: String,
}

impl KiirooCmd {
    pub fn new(device_index: u32, command: &str) -> Self {
        Self {
            id: 1,
            device_index,
            command: command.to_owned(),
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VorzeA10CycloneCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Clockwise"))]
    pub clockwise: bool,
}

impl VorzeA10CycloneCmd {
    pub fn new(device_index: u32, speed: u32, clockwise: bool) -> Self {
        Self {
            id: 1,
            device_index,
            speed,
            clockwise,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct SingleMotorVibrateCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: f64,
}

impl SingleMotorVibrateCmd {
    pub fn new(device_index: u32, speed: f64) -> Self {
        Self {
            id: 1,
            device_index,
            speed,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RawWriteCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
    pub endpoint: Endpoint,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Data"))]
    pub data: Vec<u8>,
    #[cfg_attr(feature = "serialize_json", serde(rename = "WriteWithResponse"))]
    pub write_with_response: bool,
}

impl RawWriteCmd {
    pub fn new(
        device_index: u32,
        endpoint: Endpoint,
        data: Vec<u8>,
        write_with_response: bool,
    ) -> Self {
        Self {
            id: 1,
            device_index,
            endpoint,
            data,
            write_with_response,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RawReadCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
    pub endpoint: Endpoint,
    #[cfg_attr(feature = "serialize_json", serde(rename = "ExpectedLength"))]
    pub expected_length: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Timeout"))]
    pub timeout: u32,
}

impl RawReadCmd {
    pub fn new(device_index: u32, endpoint: Endpoint, expected_length: u32, timeout: u32) -> Self {
        Self {
            id: 1,
            device_index,
            endpoint,
            expected_length,
            timeout,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct RawReading {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
    pub endpoint: Endpoint,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Data"))]
    pub data: Vec<u8>,
}

impl RawReading {
    pub fn new(device_index: u32, endpoint: Endpoint, data: Vec<u8>) -> Self {
        Self {
            id: 1,
            device_index,
            endpoint,
            data,
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct SubscribeCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
    pub endpoint: Endpoint,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MessageType"))]
    pub message_type: String,
}

impl SubscribeCmd {
    pub fn new(device_index: u32, endpoint: Endpoint, message_type: &str) -> Self {
        Self {
            id: 1,
            device_index,
            endpoint,
            message_type: message_type.to_owned(),
        }
    }
}

#[derive(Debug, ButtplugDeviceMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct UnsubscribeCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Endpoint"))]
    pub endpoint: Endpoint,
    #[cfg_attr(feature = "serialize_json", serde(rename = "MessageType"))]
    pub message_type: String,
}

impl UnsubscribeCmd {
    pub fn new(device_index: u32, endpoint: Endpoint, message_type: &str) -> Self {
        Self {
            id: 1,
            device_index,
            endpoint,
            message_type: message_type.to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, ButtplugMessage, ToSpecificButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
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
    StopAllDevices(StopAllDevices),
    VibrateCmd(VibrateCmd),
    LinearCmd(LinearCmd),
    RotateCmd(RotateCmd),
    FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
    LovenseCmd(LovenseCmd),
    KiirooCmd(KiirooCmd),
    VorzeA10CycloneCmd(VorzeA10CycloneCmd),
    SingleMotorVibrateCmd(SingleMotorVibrateCmd),
    RawWriteCmd(RawWriteCmd),
    RawReadCmd(RawReadCmd),
    StopDeviceCmd(StopDeviceCmd),
    RawReading(RawReading),
    SubscribeCmd(SubscribeCmd),
    UnsubscribeCmd(UnsubscribeCmd),
}

/// Messages that should never be received from the client.
#[derive(
    Debug, Clone, PartialEq, ButtplugMessage, TryFromButtplugMessageUnion, ToSpecificButtplugMessage,
)]
pub enum ButtplugSystemMessageUnion {
    Ok(Ok),
    Error(Error),
    Log(Log),
    ServerInfo(ServerInfo),
    DeviceList(DeviceList),
    DeviceAdded(DeviceAdded),
    DeviceRemoved(DeviceRemoved),
    ScanningFinished(ScanningFinished),
    RawReading(RawReading),
}

/// Messages that should never be received from the client.
#[derive(
    Debug, Clone, PartialEq, ButtplugMessage, TryFromButtplugMessageUnion, ToSpecificButtplugMessage,
)]
pub enum ButtplugDeviceManagerMessageUnion {
    RequestDeviceList(RequestDeviceList),
    StopAllDevices(StopAllDevices),
    StartScanning(StartScanning),
    StopScanning(StopScanning),
}

/// Messages that should be routed to device instances.
#[derive(
    Debug,
    Clone,
    PartialEq,
    ButtplugDeviceMessage,
    TryFromButtplugMessageUnion,
    ToSpecificButtplugMessage,
)]
pub enum ButtplugDeviceCommandMessageUnion {
    VibrateCmd(VibrateCmd),
    LinearCmd(LinearCmd),
    RotateCmd(RotateCmd),
    FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
    LovenseCmd(LovenseCmd),
    KiirooCmd(KiirooCmd),
    VorzeA10CycloneCmd(VorzeA10CycloneCmd),
    SingleMotorVibrateCmd(SingleMotorVibrateCmd),
    RawWriteCmd(RawWriteCmd),
    RawReadCmd(RawReadCmd),
    StopDeviceCmd(StopDeviceCmd),
    SubscribeCmd(SubscribeCmd),
    UnsubscribeCmd(UnsubscribeCmd),
}

#[cfg(feature = "serialize_json")]
#[cfg(test)]
mod test {
    use super::{ButtplugMessage, ButtplugMessageUnion, Error, ErrorCode, Ok, RawReading};
    use crate::device::Endpoint;

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
    fn test_protocol_json() {
        const PROTOCOL_STR: &str = "[{\"Ok\":{\"Id\":0}}]";
        let ok = ButtplugMessageUnion::Ok(Ok::new(0));
        let js = ok.as_protocol_json();
        assert_eq!(PROTOCOL_STR, js);
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

    #[test]
    fn test_endpoint_deserialize() {
        let endpoint_str =
            "{\"RawReading\":{\"Id\":1,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
        let union: ButtplugMessageUnion = serde_json::from_str(&endpoint_str).unwrap();
        assert_eq!(
            ButtplugMessageUnion::RawReading(RawReading::new(0, Endpoint::Tx, vec!(0))),
            union
        );
    }

    #[test]
    fn test_endpoint_serialize() {
        let union = ButtplugMessageUnion::RawReading(RawReading::new(0, Endpoint::Tx, vec![0]));
        let js = serde_json::to_string(&union).unwrap();
        println!("{}", js);
        let endpoint_str =
            "{\"RawReading\":{\"Id\":1,\"DeviceIndex\":0,\"Endpoint\":\"tx\",\"Data\":[0]}}";
        assert_eq!(js, endpoint_str);
    }
}
