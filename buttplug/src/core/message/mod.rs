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
//! with only the current message spec being included here (older message specs are only used for
//! backward compatibilty and are in the server::message module). There are also enum types that are
//! used to classify messages into categories, for instance, messages that only should be sent by a
//! client or server.

pub mod v0;
pub mod v1;
pub mod v2;
pub mod v4;

mod device_feature;
mod endpoint;
pub mod serializer;

pub use device_feature::*;
pub use endpoint::Endpoint;
pub use v0::*;
pub use v1::*;
pub use v2::*;
pub use v4::*;

use crate::core::errors::ButtplugMessageError;
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize-json")]
use serde_repr::{Deserialize_repr, Serialize_repr};
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

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugActuatorFeatureMessageType {
  ValueCmd,
  ValueWithParameterCmd,
}

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugSensorFeatureMessageType {
  SensorReadCmd,
  SensorSubscribeCmd,
}

#[derive(Copy, Debug, Clone, Hash, Display, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugRawFeatureMessageType {
  RawReadCmd,
  RawWriteCmd,
  RawSubscribeCmd,
  RawUnsubscribeCmd,
}

/// Type alias for the latest version of client-to-server messages.
pub type ButtplugClientMessageCurrent = ButtplugClientMessageV4;
/// Type alias for the latest version of server-to-client messages.
pub type ButtplugServerMessageCurrent = ButtplugServerMessageV4;

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
  Heater,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  PositionWithDuration,
}

impl TryFrom<FeatureType> for ActuatorType {
  type Error = String;
  fn try_from(value: FeatureType) -> Result<Self, Self::Error> {
    match value {
      FeatureType::Unknown => Ok(ActuatorType::Unknown),
      FeatureType::Vibrate => Ok(ActuatorType::Vibrate),
      FeatureType::Rotate => Ok(ActuatorType::Rotate),
      FeatureType::Heater => Ok(ActuatorType::Heater),
      FeatureType::RotateWithDirection => Ok(ActuatorType::RotateWithDirection),
      FeatureType::PositionWithDuration => Ok(ActuatorType::PositionWithDuration),
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
