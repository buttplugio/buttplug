// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representations of low level [Buttplug Protocol](https://buttplug-spec.docs.buttplug.io)
//! messages
//!
//! The core communication types for the Buttplug protocol. There are structs for each message type,
//! sometimes with multiple versions of the same message relating to different spec versions. There
//! are also enum types that are used to classify messages into categories, for instance, messages
//! that only should be sent by a client or server.

mod battery_level_cmd;
mod battery_level_reading;
mod client_device_message_attributes;
mod device_added;
mod device_feature;
mod device_list;
mod device_message_info;
mod device_removed;
mod endpoint;
mod error;
mod fleshlight_launch_fw12_cmd;
mod kiiroo_cmd;
mod linear_cmd;
mod log;
mod log_level;
mod lovense_cmd;
mod ok;
mod ping;
mod raw_read_cmd;
mod raw_reading;
mod raw_subscribe_cmd;
mod raw_unsubscribe_cmd;
mod raw_write_cmd;
mod request_device_list;
mod request_log;
mod request_server_info;
mod rotate_cmd;
mod rssi_level_cmd;
mod rssi_level_reading;
mod scalar_cmd;
mod scanning_finished;
mod sensor_read_cmd;
mod sensor_reading;
mod sensor_subscribe_cmd;
mod sensor_unsubscribe_cmd;
pub mod serializer;
mod server_info;
mod single_motor_vibrate_cmd;
mod start_scanning;
mod stop_all_devices;
mod stop_device_cmd;
mod stop_scanning;
mod test;
mod vibrate_cmd;
mod vorze_a10_cyclone_cmd;

pub use self::log::LogV0;
pub use battery_level_cmd::BatteryLevelCmdV2;
pub use battery_level_reading::BatteryLevelReadingV2;
pub use client_device_message_attributes::{
  ActuatorType,
  ClientDeviceMessageAttributesV1,
  ClientDeviceMessageAttributesV2,
  ClientDeviceMessageAttributesV3,
  ClientDeviceMessageAttributesV3Builder,
  ClientGenericDeviceMessageAttributesV3,
  NullDeviceMessageAttributesV1,
  RawDeviceMessageAttributesV2,
  SensorDeviceMessageAttributesV3,
  SensorType,
};
pub use device_added::{DeviceAddedV0, DeviceAddedV1, DeviceAddedV2, DeviceAddedV3, DeviceAddedV4};
pub use device_feature::{
  DeviceFeature,
  DeviceFeatureActuator,
  DeviceFeatureRaw,
  DeviceFeatureSensor,
  FeatureType,
};
pub use device_list::{DeviceListV0, DeviceListV1, DeviceListV2, DeviceListV3, DeviceListV4};
pub use device_message_info::{
  DeviceMessageInfoV0,
  DeviceMessageInfoV1,
  DeviceMessageInfoV2,
  DeviceMessageInfoV3,
  DeviceMessageInfoV4,
};
pub use device_removed::DeviceRemovedV0;
pub use endpoint::Endpoint;
pub use error::{ErrorCode, ErrorV0};
pub use fleshlight_launch_fw12_cmd::FleshlightLaunchFW12CmdV0;
pub use kiiroo_cmd::KiirooCmdV0;
pub use linear_cmd::{LinearCmdV1, LinearCmdV4, VectorSubcommandV1, VectorSubcommandV4};
pub use log_level::LogLevel;
pub use lovense_cmd::LovenseCmdV0;
pub use ok::OkV0;
pub use ping::PingV0;
pub use raw_read_cmd::RawReadCmdV2;
pub use raw_reading::RawReadingV2;
pub use raw_subscribe_cmd::RawSubscribeCmdV2;
pub use raw_unsubscribe_cmd::RawUnsubscribeCmdV2;
pub use raw_write_cmd::RawWriteCmdV2;
pub use request_device_list::RequestDeviceListV0;
pub use request_log::RequestLogV0;
pub use request_server_info::RequestServerInfoV1;
pub use rotate_cmd::{RotateCmdV1, RotateCmdV4, RotationSubcommandV1, RotationSubcommandV4};
pub use rssi_level_cmd::RSSILevelCmdV2;
pub use rssi_level_reading::RSSILevelReadingV2;
pub use scalar_cmd::{ScalarCmdV3, ScalarCmdV4, ScalarSubcommandV3, ScalarSubcommandV4};
pub use scanning_finished::ScanningFinishedV0;
pub use sensor_read_cmd::{SensorReadCmdV3, SensorReadCmdV4};
pub use sensor_reading::{SensorReadingV3, SensorReadingV4};
pub use sensor_subscribe_cmd::{SensorSubscribeCmdV3, SensorSubscribeCmdV4};
pub use sensor_unsubscribe_cmd::{SensorUnsubscribeCmdV3, SensorUnsubscribeCmdV4};
pub use server_info::{ServerInfoV0, ServerInfoV2};
pub use single_motor_vibrate_cmd::SingleMotorVibrateCmdV0;
pub use start_scanning::StartScanningV0;
pub use stop_all_devices::StopAllDevicesV0;
pub use stop_device_cmd::StopDeviceCmdV0;
pub use stop_scanning::StopScanningV0;
pub use test::TestV0;
pub use vibrate_cmd::{VibrateCmdV1, VibrateSubcommandV1};
pub use vorze_a10_cyclone_cmd::VorzeA10CycloneCmdV0;

use crate::core::errors::ButtplugMessageError;
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize-json")]
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::cmp::Ordering;
use std::convert::TryFrom;

use super::errors::ButtplugError;

/// Enum of possible [Buttplug Message
/// Spec](https://buttplug-spec.docs.buttplug.io) versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
#[repr(u32)]
#[cfg_attr(feature = "serialize-json", derive(Serialize_repr, Deserialize_repr))]
pub enum ButtplugMessageSpecVersion {
  Version0 = 0,
  Version1 = 1,
  Version2 = 2,
  Version3 = 3,
  Version4 = 4,
}

impl TryFrom<i32> for ButtplugMessageSpecVersion {
  type Error = ButtplugError;

  // There's probably another crate to make this easier but eh.
  fn try_from(value: i32) -> Result<Self, Self::Error> {
    match value {
      0 => Ok(ButtplugMessageSpecVersion::Version0),
      1 => Ok(ButtplugMessageSpecVersion::Version1),
      2 => Ok(ButtplugMessageSpecVersion::Version2),
      3 => Ok(ButtplugMessageSpecVersion::Version3),
      4 => Ok(ButtplugMessageSpecVersion::Version4),
      _ => Err(
        ButtplugMessageError::InvalidMessageContents(format!(
          "Message spec version {} is not valid",
          value
        ))
        .into(),
      ),
    }
  }
}

/// Message Id for events sent from the server, which are not in response to a
/// client request.
pub const BUTTPLUG_SERVER_EVENT_ID: u32 = 0;

/// The current latest version of the spec implemented by the library.
pub const BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION: ButtplugMessageSpecVersion =
  ButtplugMessageSpecVersion::Version3;

pub trait ButtplugMessageFinalizer {
  fn finalize(&mut self) {
  }
}

/// Base trait for all Buttplug Protocol Message Structs. Handles management of
/// message ids, as well as implementing conveinence functions for converting
/// between message structs and various message enums, serialization, etc...
pub trait ButtplugMessage:
  ButtplugMessageValidator + ButtplugMessageFinalizer + Send + Sync + Clone
{
  /// Returns the id number of the message
  fn id(&self) -> u32;
  /// Sets the id number of the message.
  fn set_id(&mut self, id: u32);
  /// True if the message is an event (message id of 0) from the server.
  fn is_server_event(&self) -> bool {
    self.id() == BUTTPLUG_SERVER_EVENT_ID
  }
}

/// Validation function for message contents. Can be run before message is
/// transmitted, as message may be formed and mutated at multiple points in the
/// library, or may need to be checked after deserialization. Message enums will
/// run this on whatever their variant is.
pub trait ButtplugMessageValidator {
  /// Returns () if the message is valid, otherwise returns a message error.
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    // By default, return Ok, as many messages won't have any checks.
    Ok(())
  }

  fn is_system_id(&self, id: u32) -> Result<(), ButtplugMessageError> {
    if id == 0 {
      Ok(())
    } else {
      Err(ButtplugMessageError::InvalidMessageContents(
        "Message should have id of 0, as it is a system message.".to_string(),
      ))
    }
  }

  fn is_not_system_id(&self, id: u32) -> Result<(), ButtplugMessageError> {
    if id == 0 {
      Err(ButtplugMessageError::InvalidMessageContents(
        "Message should not have 0 for an Id. Id of 0 is reserved for system messages.".to_string(),
      ))
    } else {
      Ok(())
    }
  }

  fn is_in_command_range(&self, value: f64, error_msg: String) -> Result<(), ButtplugMessageError> {
    if !(0.0..=1.0).contains(&value) {
      Err(ButtplugMessageError::InvalidMessageContents(error_msg))
    } else {
      Ok(())
    }
  }
}

/// Adds device index handling to the [ButtplugMessage] trait.
pub trait ButtplugDeviceMessage: ButtplugMessage {
  fn device_index(&self) -> u32;
  fn set_device_index(&mut self, id: u32);
}

/// Used in [MessageAttributes][crate::core::messages::DeviceMessageAttributes] for denoting message
/// capabilties.
#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ButtplugDeviceMessageType {
  VibrateCmd,
  LinearCmd,
  RotateCmd,
  StopDeviceCmd,
  RawWriteCmd,
  RawReadCmd,
  RawSubscribeCmd,
  RawUnsubscribeCmd,
  BatteryLevelCmd,
  RSSILevelCmd,
  ScalarCmd,
  SensorReadCmd,
  SensorSubscribeCmd,
  SensorUnsubscribeCmd,
  // Deprecated generic commands
  SingleMotorVibrateCmd,
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd,
  LovenseCmd,
  KiirooCmd,
  VorzeA10CycloneCmd,
}

// Ordering for ButtplugDeviceMessageType should be lexicographic, for
// serialization reasons.
impl PartialOrd for ButtplugDeviceMessageType {
  fn partial_cmp(&self, other: &ButtplugDeviceMessageType) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for ButtplugDeviceMessageType {
  fn cmp(&self, other: &ButtplugDeviceMessageType) -> Ordering {
    self.to_string().cmp(&other.to_string())
  }
}

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugActuatorFeatureMessageType {
  ScalarCmd,
  RotateCmd,
  LinearCmd,
}

impl From<ButtplugActuatorFeatureMessageType> for ButtplugDeviceMessageType {
  fn from(value: ButtplugActuatorFeatureMessageType) -> Self {
    match value {
      ButtplugActuatorFeatureMessageType::LinearCmd => ButtplugDeviceMessageType::LinearCmd,
      ButtplugActuatorFeatureMessageType::RotateCmd => ButtplugDeviceMessageType::RotateCmd,
      ButtplugActuatorFeatureMessageType::ScalarCmd => ButtplugDeviceMessageType::ScalarCmd,
    }
  }
}

impl TryFrom<ButtplugDeviceMessageType> for ButtplugActuatorFeatureMessageType {
  type Error = ();

  fn try_from(value: ButtplugDeviceMessageType) -> Result<Self, Self::Error> {
    match value {
      ButtplugDeviceMessageType::LinearCmd => Ok(ButtplugActuatorFeatureMessageType::LinearCmd),
      ButtplugDeviceMessageType::RotateCmd => Ok(ButtplugActuatorFeatureMessageType::RotateCmd),
      ButtplugDeviceMessageType::ScalarCmd => Ok(ButtplugActuatorFeatureMessageType::ScalarCmd),
      _ => Err(()),
    }
  }
}

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugSensorFeatureMessageType {
  SensorReadCmd,
  SensorSubscribeCmd,
}

impl From<ButtplugSensorFeatureMessageType> for ButtplugDeviceMessageType {
  fn from(value: ButtplugSensorFeatureMessageType) -> Self {
    match value {
      ButtplugSensorFeatureMessageType::SensorReadCmd => ButtplugDeviceMessageType::SensorReadCmd,
      ButtplugSensorFeatureMessageType::SensorSubscribeCmd => {
        ButtplugDeviceMessageType::SensorSubscribeCmd
      }
    }
  }
}

impl TryFrom<ButtplugDeviceMessageType> for ButtplugSensorFeatureMessageType {
  type Error = ();

  fn try_from(value: ButtplugDeviceMessageType) -> Result<Self, Self::Error> {
    match value {
      ButtplugDeviceMessageType::SensorReadCmd => {
        Ok(ButtplugSensorFeatureMessageType::SensorReadCmd)
      }
      ButtplugDeviceMessageType::SensorSubscribeCmd => {
        Ok(ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
      }
      _ => Err(()),
    }
  }
}

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugRawFeatureMessageType {
  RawReadCmd,
  RawWriteCmd,
  RawSubscribeCmd,
}

impl From<ButtplugRawFeatureMessageType> for ButtplugDeviceMessageType {
  fn from(value: ButtplugRawFeatureMessageType) -> Self {
    match value {
      ButtplugRawFeatureMessageType::RawReadCmd => ButtplugDeviceMessageType::RawReadCmd,
      ButtplugRawFeatureMessageType::RawWriteCmd => ButtplugDeviceMessageType::RawWriteCmd,
      ButtplugRawFeatureMessageType::RawSubscribeCmd => ButtplugDeviceMessageType::RawSubscribeCmd,
    }
  }
}

impl TryFrom<ButtplugDeviceMessageType> for ButtplugRawFeatureMessageType {
  type Error = ();

  fn try_from(value: ButtplugDeviceMessageType) -> Result<Self, Self::Error> {
    match value {
      ButtplugDeviceMessageType::RawReadCmd => Ok(ButtplugRawFeatureMessageType::RawReadCmd),
      ButtplugDeviceMessageType::RawWriteCmd => Ok(ButtplugRawFeatureMessageType::RawWriteCmd),
      ButtplugDeviceMessageType::RawSubscribeCmd => {
        Ok(ButtplugRawFeatureMessageType::RawSubscribeCmd)
      }
      _ => Err(()),
    }
  }
}

#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator,
)]
pub enum ButtplugClientMessageVariant {
  V0(ButtplugClientMessageV0),
  V1(ButtplugClientMessageV1),
  V2(ButtplugClientMessageV2),
  V3(ButtplugClientMessageV3),
  V4(ButtplugClientMessageV4),
}

impl ButtplugClientMessageVariant {
  pub fn version(&self) -> ButtplugMessageSpecVersion {
    match self {
      Self::V0(_) => ButtplugMessageSpecVersion::Version0,
      Self::V1(_) => ButtplugMessageSpecVersion::Version1,
      Self::V2(_) => ButtplugMessageSpecVersion::Version2,
      Self::V3(_) => ButtplugMessageSpecVersion::Version3,
      Self::V4(_) => ButtplugMessageSpecVersion::Version4,
    }
  }
}

impl From<ButtplugClientMessageV0> for ButtplugClientMessageVariant {
  fn from(value: ButtplugClientMessageV0) -> Self {
    ButtplugClientMessageVariant::V0(value)
  }
}

impl From<ButtplugClientMessageV1> for ButtplugClientMessageVariant {
  fn from(value: ButtplugClientMessageV1) -> Self {
    ButtplugClientMessageVariant::V1(value)
  }
}

impl From<ButtplugClientMessageV2> for ButtplugClientMessageVariant {
  fn from(value: ButtplugClientMessageV2) -> Self {
    ButtplugClientMessageVariant::V2(value)
  }
}

impl From<ButtplugClientMessageV3> for ButtplugClientMessageVariant {
  fn from(value: ButtplugClientMessageV3) -> Self {
    ButtplugClientMessageVariant::V3(value)
  }
}

impl From<ButtplugClientMessageV4> for ButtplugClientMessageVariant {
  fn from(value: ButtplugClientMessageV4) -> Self {
    ButtplugClientMessageVariant::V4(value)
  }
}

#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator,
)]
pub enum ButtplugServerMessageVariant {
  V0(ButtplugServerMessageV0),
  V1(ButtplugServerMessageV1),
  V2(ButtplugServerMessageV2),
  V3(ButtplugServerMessageV3),
  V4(ButtplugServerMessageV4),
}

impl ButtplugServerMessageVariant {
  pub fn version(&self) -> ButtplugMessageSpecVersion {
    match self {
      Self::V0(_) => ButtplugMessageSpecVersion::Version0,
      Self::V1(_) => ButtplugMessageSpecVersion::Version1,
      Self::V2(_) => ButtplugMessageSpecVersion::Version2,
      Self::V3(_) => ButtplugMessageSpecVersion::Version3,
      Self::V4(_) => ButtplugMessageSpecVersion::Version4,
    }
  }
}

impl From<ButtplugServerMessageV0> for ButtplugServerMessageVariant {
  fn from(value: ButtplugServerMessageV0) -> Self {
    ButtplugServerMessageVariant::V0(value)
  }
}

impl From<ButtplugServerMessageV1> for ButtplugServerMessageVariant {
  fn from(value: ButtplugServerMessageV1) -> Self {
    ButtplugServerMessageVariant::V1(value)
  }
}

impl From<ButtplugServerMessageV2> for ButtplugServerMessageVariant {
  fn from(value: ButtplugServerMessageV2) -> Self {
    ButtplugServerMessageVariant::V2(value)
  }
}

impl From<ButtplugServerMessageV3> for ButtplugServerMessageVariant {
  fn from(value: ButtplugServerMessageV3) -> Self {
    ButtplugServerMessageVariant::V3(value)
  }
}

impl From<ButtplugServerMessageV4> for ButtplugServerMessageVariant {
  fn from(value: ButtplugServerMessageV4) -> Self {
    ButtplugServerMessageVariant::V4(value)
  }
}

/// Represents all possible messages a [ButtplugServer][crate::server::ButtplugServer] can send to a
/// [ButtplugClient][crate::client::ButtplugClient] that denote an EVENT from a device. These are
/// only used in notifications, so read requests will not need to be added here, only messages that
/// will require Id of 0.
#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugServerDeviceMessage {
  // Generic commands
  RawReading(RawReadingV2),
  // Generic Sensor Reading Messages
  SensorReading(SensorReadingV4),
}

impl From<ButtplugServerDeviceMessage> for ButtplugServerMessageV4 {
  fn from(other: ButtplugServerDeviceMessage) -> Self {
    match other {
      ButtplugServerDeviceMessage::RawReading(msg) => ButtplugServerMessageV4::RawReading(msg),
      ButtplugServerDeviceMessage::SensorReading(msg) => {
        ButtplugServerMessageV4::SensorReading(msg)
      }
    }
  }
}

/// Type alias for the latest version of client-to-server messages.
pub type ButtplugClientMessageCurrent = ButtplugClientMessageV3;
/// Type alias for the latest version of server-to-client messages.
pub type ButtplugServerMessageCurrent = ButtplugServerMessageV3;

/// Represents all client-to-server messages in v3 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  ScalarCmd(ScalarCmdV4),
  LinearCmd(LinearCmdV4),
  RotateCmd(RotateCmdV4),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
  // Sensor commands
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
}

/// Represents all server-to-client messages in v3 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageValidator, FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV4 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV4),
  DeviceAdded(DeviceAddedV4),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Generic commands
  RawReading(RawReadingV2),
  // Sensor commands
  SensorReading(SensorReadingV4),
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV4 {
  fn finalize(&mut self) {
    match self {
      ButtplugServerMessageV4::DeviceAdded(da) => da.finalize(),
      ButtplugServerMessageV4::DeviceList(dl) => dl.finalize(),
      _ => return,
    }
  }
}

/// Represents all client-to-server messages in v3 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV3 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  VibrateCmd(VibrateCmdV1),
  LinearCmd(LinearCmdV1),
  RotateCmd(RotateCmdV1),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  StopDeviceCmd(StopDeviceCmdV0),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
  ScalarCmd(ScalarCmdV3),
  // Sensor commands
  SensorReadCmd(SensorReadCmdV3),
  SensorSubscribeCmd(SensorSubscribeCmdV3),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV3),
}

/// Represents all server-to-client messages in v3 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageValidator, FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV3 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV3),
  DeviceAdded(DeviceAddedV3),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Generic commands
  RawReading(RawReadingV2),
  // Sensor commands
  SensorReading(SensorReadingV3),
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV3 {
  fn finalize(&mut self) {
    match self {
      ButtplugServerMessageV3::DeviceAdded(da) => da.finalize(),
      ButtplugServerMessageV3::DeviceList(dl) => dl.finalize(),
      _ => return,
    }
  }
}

/// Represents all client-to-server messages in v2 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV2 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  VibrateCmd(VibrateCmdV1),
  LinearCmd(LinearCmdV1),
  RotateCmd(RotateCmdV1),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  StopDeviceCmd(StopDeviceCmdV0),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
  // Sensor commands
  BatteryLevelCmd(BatteryLevelCmdV2),
  RSSILevelCmd(RSSILevelCmdV2),
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV2 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV2),
  DeviceAdded(DeviceAddedV2),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Generic commands
  RawReading(RawReadingV2),
  // Sensor commands
  BatteryLevelReading(BatteryLevelReadingV2),
  RSSILevelReading(RSSILevelReadingV2),
}

/// Represents all client-to-server messages in v1 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV1 {
  // Handshake and server messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  RequestLog(RequestLogV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  VibrateCmd(VibrateCmdV1),
  LinearCmd(LinearCmdV1),
  RotateCmd(RotateCmdV1),
  StopDeviceCmd(StopDeviceCmdV0),
  // Deprecated generic commands (not removed until v2)
  SingleMotorVibrateCmd(SingleMotorVibrateCmdV0),
  // Deprecated device specific commands (not removed until v2)
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12CmdV0),
  LovenseCmd(LovenseCmdV0),
  KiirooCmd(KiirooCmdV0),
  VorzeA10CycloneCmd(VorzeA10CycloneCmdV0),
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV1 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  Log(LogV0),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV1),
  DeviceAdded(DeviceAddedV1),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
}

/// Represents all client-to-server messages in v0 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV0 {
  RequestLog(RequestLogV0),
  Ping(PingV0),
  // Handshake messages
  //
  // We use RequestServerInfoV1 here, as the only difference between v0 and v1 was passing the spec
  // version. If the spec version doesn't exist, we automatically set the spec version to 0.
  RequestServerInfo(RequestServerInfoV1),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  StopDeviceCmd(StopDeviceCmdV0),
  // Deprecated generic commands
  SingleMotorVibrateCmd(SingleMotorVibrateCmdV0),
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12CmdV0),
  LovenseCmd(LovenseCmdV0),
  KiirooCmd(KiirooCmdV0),
  VorzeA10CycloneCmd(VorzeA10CycloneCmdV0),
}

/// Represents all server-to-client messages in v0 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageValidator, ButtplugMessageFinalizer,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV0 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  Log(LogV0),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV0),
  DeviceAdded(DeviceAddedV0),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
}

/// Represents messages that should go to the
/// [DeviceManager][crate::server::device_manager::DeviceManager] of a
/// [ButtplugServer](crate::server::ButtplugServer)
#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugDeviceManagerMessageUnion {
  RequestDeviceList(RequestDeviceListV0),
  StopAllDevices(StopAllDevicesV0),
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
}

impl TryFrom<ButtplugClientMessageV4> for ButtplugDeviceManagerMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::RequestDeviceList(m))
      }
      ButtplugClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopAllDevices(m))
      }
      ButtplugClientMessageV4::StartScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StartScanning(m))
      }
      ButtplugClientMessageV4::StopScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopScanning(m))
      }
      _ => Err(()),
    }
  }
}

/// Represents all possible device command message types.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugDeviceMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugDeviceCommandMessageUnion {
  StopDeviceCmd(StopDeviceCmdV0),
  LinearCmd(LinearCmdV4),
  RotateCmd(RotateCmdV4),
  ScalarCmd(ScalarCmdV4),
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

impl TryFrom<ButtplugClientMessageV4> for ButtplugDeviceCommandMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV4::StopDeviceCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::StopDeviceCmd(m))
      }
      ButtplugClientMessageV4::LinearCmd(m) => Ok(ButtplugDeviceCommandMessageUnion::LinearCmd(m)),
      ButtplugClientMessageV4::RotateCmd(m) => Ok(ButtplugDeviceCommandMessageUnion::RotateCmd(m)),
      ButtplugClientMessageV4::ScalarCmd(m) => Ok(ButtplugDeviceCommandMessageUnion::ScalarCmd(m)),
      ButtplugClientMessageV4::SensorReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorReadCmd(m))
      }
      ButtplugClientMessageV4::SensorSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(m))
      }
      ButtplugClientMessageV4::SensorUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(m))
      }
      ButtplugClientMessageV4::RawWriteCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawWriteCmd(m))
      }
      ButtplugClientMessageV4::RawReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawReadCmd(m))
      }
      ButtplugClientMessageV4::RawSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV4::RawUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(m))
      }
      _ => Err(()),
    }
  }
}
