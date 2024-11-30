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

pub mod v0;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

mod device_feature;
mod endpoint;
pub mod serializer;

use std::collections::HashMap;
pub use device_feature::*;
pub use endpoint::Endpoint;
pub use v0::*;
pub use v1::*;
pub use v2::*;
pub use v3::*;
pub use v4::*;

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

#[cfg(not(feature = "default_v4_spec"))]
/// The current latest version of the spec implemented by the library.
pub const BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION: ButtplugMessageSpecVersion =
  ButtplugMessageSpecVersion::Version3;
#[cfg(feature = "default_v4_spec")]
/// The current latest version of the spec implemented by the library.
pub const BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION: ButtplugMessageSpecVersion =
  ButtplugMessageSpecVersion::Version4;
  
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
  LevelCmd,
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
  LevelCmd,
  LinearCmd,
}

impl From<ButtplugActuatorFeatureMessageType> for ButtplugDeviceMessageType {
  fn from(value: ButtplugActuatorFeatureMessageType) -> Self {
    match value {
      ButtplugActuatorFeatureMessageType::LinearCmd => ButtplugDeviceMessageType::LinearCmd,
      ButtplugActuatorFeatureMessageType::LevelCmd => ButtplugDeviceMessageType::ScalarCmd,
    }
  }
}

impl TryFrom<ButtplugDeviceMessageType> for ButtplugActuatorFeatureMessageType {
  type Error = ();

  fn try_from(value: ButtplugDeviceMessageType) -> Result<Self, Self::Error> {
    match value {
      ButtplugDeviceMessageType::LinearCmd => Ok(ButtplugActuatorFeatureMessageType::LinearCmd),
      ButtplugDeviceMessageType::LevelCmd => Ok(ButtplugActuatorFeatureMessageType::LevelCmd),
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

  pub fn device_index(&self) -> Option<u32> {
    // TODO there has to be a better way to do this. We just need to dig through our enum and see if
    // our message impls ButtplugDeviceMessage. Manually doing this works but is so gross.
    match self {
      Self::V0(msg) => match msg {
        ButtplugClientMessageV0::FleshlightLaunchFW12Cmd(a) => Some(a.device_index()),
        ButtplugClientMessageV0::KiirooCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV0::LovenseCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV0::SingleMotorVibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV0::VorzeA10CycloneCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V1(msg) => match msg {
        ButtplugClientMessageV1::FleshlightLaunchFW12Cmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::KiirooCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::LovenseCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::SingleMotorVibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::VorzeA10CycloneCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::VibrateCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V2(msg) => match msg {
        ButtplugClientMessageV2::VibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RSSILevelCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RotateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::LinearCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::BatteryLevelCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RawReadCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RawWriteCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RawSubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RawUnsubscribeCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V3(msg) => match msg {
        ButtplugClientMessageV3::VibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::SensorSubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::SensorUnsubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::ScalarCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::RotateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::LinearCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::SensorReadCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::RawReadCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::RawWriteCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::RawSubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV3::RawUnsubscribeCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V4(msg) => match msg {
        ButtplugClientMessageV4::SensorSubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::SensorUnsubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::LevelCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::LinearCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::SensorReadCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::RawReadCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::RawWriteCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::RawSubscribeCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::RawUnsubscribeCmd(a) => Some(a.device_index()),
        _ => None,
      },
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
pub(crate) enum ButtplugDeviceManagerMessageUnion {
  RequestDeviceList(RequestDeviceListV0),
  StopAllDevices(StopAllDevicesV0),
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
}

impl TryFrom<ButtplugInternalClientMessageV4> for ButtplugDeviceManagerMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugInternalClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugInternalClientMessageV4::RequestDeviceList(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::RequestDeviceList(m))
      }
      ButtplugInternalClientMessageV4::StopAllDevices(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StopAllDevices(m))
      }
      ButtplugInternalClientMessageV4::StartScanning(m) => {
        Ok(ButtplugDeviceManagerMessageUnion::StartScanning(m))
      }
      ButtplugInternalClientMessageV4::StopScanning(m) => {
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
pub enum ButtplugDeviceCommandMessageUnion {
  StopDeviceCmd(StopDeviceCmdV0),
  LinearCmd(LinearCmdV4),
  LevelCmd(InternalLevelCmdV4),
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

impl TryFrom<ButtplugInternalClientMessageV4> for ButtplugDeviceCommandMessageUnion {
  type Error = ();

  fn try_from(value: ButtplugInternalClientMessageV4) -> Result<Self, Self::Error> {
    match value {
      ButtplugInternalClientMessageV4::StopDeviceCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::StopDeviceCmd(m))
      }
      ButtplugInternalClientMessageV4::LinearCmd(m) => Ok(ButtplugDeviceCommandMessageUnion::LinearCmd(m)),
      ButtplugInternalClientMessageV4::LevelCmd(m) => Ok(ButtplugDeviceCommandMessageUnion::LevelCmd(m)),
      ButtplugInternalClientMessageV4::SensorReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorReadCmd(m))
      }
      ButtplugInternalClientMessageV4::SensorSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::SensorUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::RawWriteCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawWriteCmd(m))
      }
      ButtplugInternalClientMessageV4::RawReadCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawReadCmd(m))
      }
      ButtplugInternalClientMessageV4::RawSubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(m))
      }
      ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m) => {
        Ok(ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(m))
      }
      _ => Err(()),
    }
  }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorType {
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  // Two Direction Rotation Speed
  RotateWithDirection,
  Oscillate,
  Constrict,
  Inflate,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
}

impl TryFrom<FeatureType> for ActuatorType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(ActuatorType::Unknown),
      FeatureType::Vibrate => Ok(ActuatorType::Vibrate),
      FeatureType::Rotate => Ok(ActuatorType::Rotate),
      FeatureType::RotateWithDirection => Ok(ActuatorType::RotateWithDirection),
      FeatureType::Oscillate => Ok(ActuatorType::Oscillate),
      FeatureType::Constrict => Ok(ActuatorType::Constrict),
      FeatureType::Inflate => Ok(ActuatorType::Inflate),
      FeatureType::Position => Ok(ActuatorType::Position),
      _ => Err(format!(
        "Feature type {value} not valid for ActuatorType conversion"
      )),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum SensorType {
  Unknown,
  Battery,
  RSSI,
  Button,
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
}

impl TryFrom<FeatureType> for SensorType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(SensorType::Unknown),
      FeatureType::Battery => Ok(SensorType::Battery),
      FeatureType::RSSI => Ok(SensorType::RSSI),
      FeatureType::Button => Ok(SensorType::Button),
      FeatureType::Pressure => Ok(SensorType::Pressure),
      _ => Err(format!(
        "Feature type {value} not valid for SensorType conversion"
      )),
    }
  }
}

pub(crate) trait TryFromClientMessage<T>
where
  Self: Sized,
{
  fn try_from_client_message(
    msg: T,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError>;
}
