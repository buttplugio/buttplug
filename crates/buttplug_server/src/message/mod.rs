// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::{
  errors::{ButtplugError, ButtplugMessageError},
  message::{
    ButtplugClientMessageV4,
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageSpecVersion,
    ButtplugMessageValidator,
    ButtplugServerMessageV4,
    InputReadingV4,
  },
};
use server_device_attributes::ServerDeviceAttributes;

/// Macro for implementing ButtplugMessage and ButtplugMessageValidator on message enums
/// that dispatch to their inner message types. ButtplugMessageFinalizer must be implemented
/// separately as some enums have custom finalize() implementations.
macro_rules! impl_message_enum_traits {
  ($enum_name:ident { $($variant:ident),* $(,)? }) => {
    impl buttplug_core::message::ButtplugMessage for $enum_name {
      fn id(&self) -> u32 {
        match self {
          $(Self::$variant(msg) => msg.id(),)*
        }
      }
      fn set_id(&mut self, id: u32) {
        match self {
          $(Self::$variant(msg) => msg.set_id(id),)*
        }
      }
    }

    impl buttplug_core::message::ButtplugMessageValidator for $enum_name {
      fn is_valid(&self) -> Result<(), buttplug_core::errors::ButtplugMessageError> {
        match self {
          $(Self::$variant(msg) => msg.is_valid(),)*
        }
      }
    }
  };
}

/// Helper macro to extract device_index from versioned message enums.
/// Returns Some(device_index) for device commands, None for other messages.
macro_rules! extract_device_index {
  ($msg:expr, $enum_type:ident, [$($variant:ident),* $(,)?]) => {
    match $msg {
      $($enum_type::$variant(a) => Some(a.device_index()),)*
      _ => None,
    }
  };
}

pub mod serializer;
pub mod server_device_attributes;
mod v0;
mod v1;
mod v2;
mod v3;
mod v4;

pub use v0::*;
pub use v1::*;
pub use v2::*;
pub use v3::*;
pub use v4::*;

#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum ButtplugClientMessageVariant {
  V0(ButtplugClientMessageV0),
  V1(ButtplugClientMessageV1),
  V2(ButtplugClientMessageV2),
  V3(ButtplugClientMessageV3),
  V4(ButtplugClientMessageV4),
}

impl_message_enum_traits!(ButtplugClientMessageVariant { V0, V1, V2, V3, V4 });
impl ButtplugMessageFinalizer for ButtplugClientMessageVariant {
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
    match self {
      Self::V0(msg) => extract_device_index!(
        msg,
        ButtplugClientMessageV0,
        [
          FleshlightLaunchFW12Cmd,
          SingleMotorVibrateCmd,
          VorzeA10CycloneCmd
        ]
      ),
      Self::V1(msg) => extract_device_index!(
        msg,
        ButtplugClientMessageV1,
        [
          FleshlightLaunchFW12Cmd,
          SingleMotorVibrateCmd,
          VorzeA10CycloneCmd,
          VibrateCmd
        ]
      ),
      Self::V2(msg) => extract_device_index!(
        msg,
        ButtplugClientMessageV2,
        [VibrateCmd, RotateCmd, LinearCmd, BatteryLevelCmd]
      ),
      Self::V3(msg) => extract_device_index!(
        msg,
        ButtplugClientMessageV3,
        [
          VibrateCmd,
          SensorSubscribeCmd,
          SensorUnsubscribeCmd,
          ScalarCmd,
          RotateCmd,
          LinearCmd,
          SensorReadCmd
        ]
      ),
      Self::V4(msg) => extract_device_index!(msg, ButtplugClientMessageV4, [OutputCmd, InputCmd]),
    }
  }
}

#[derive(Debug, Clone, derive_more::From)]
pub enum ButtplugServerMessageVariant {
  V0(ButtplugServerMessageV0),
  V1(ButtplugServerMessageV1),
  V2(ButtplugServerMessageV2),
  V3(ButtplugServerMessageV3),
  V4(ButtplugServerMessageV4),
}

impl_message_enum_traits!(ButtplugServerMessageVariant { V0, V1, V2, V3, V4 });
impl ButtplugMessageFinalizer for ButtplugServerMessageVariant {
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

/// Represents all possible messages a [ButtplugServer][crate::server::ButtplugServer] can send to a
/// [ButtplugClient][crate::client::ButtplugClient] that denote an EVENT from a device. These are
/// only used in notifications, so read requests will not need to be added here, only messages that
/// will require Id of 0.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum ButtplugServerDeviceMessage {
  // Generic Sensor Reading Messages
  SensorReading(InputReadingV4),
}

impl_message_enum_traits!(ButtplugServerDeviceMessage { SensorReading });
impl ButtplugMessageFinalizer for ButtplugServerDeviceMessage {
}

impl From<ButtplugServerDeviceMessage> for ButtplugServerMessageV4 {
  fn from(other: ButtplugServerDeviceMessage) -> Self {
    match other {
      ButtplugServerDeviceMessage::SensorReading(msg) => ButtplugServerMessageV4::InputReading(msg),
    }
  }
}

/// TryFrom for Buttplug Device Messages that need to use a device feature definition to convert
pub(crate) trait TryFromDeviceAttributes<T>
where
  Self: Sized,
{
  fn try_from_device_attributes(
    msg: T,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, ButtplugError>;
}
