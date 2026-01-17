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

#[derive(Debug, Clone, PartialEq)]
pub enum ButtplugClientMessageVariant {
  V0(ButtplugClientMessageV0),
  V1(ButtplugClientMessageV1),
  V2(ButtplugClientMessageV2),
  V3(ButtplugClientMessageV3),
  V4(ButtplugClientMessageV4),
}

impl ButtplugMessage for ButtplugClientMessageVariant {
  fn id(&self) -> u32 {
    match self {
      ButtplugClientMessageVariant::V0(msg) => msg.id(),
      ButtplugClientMessageVariant::V1(msg) => msg.id(),
      ButtplugClientMessageVariant::V2(msg) => msg.id(),
      ButtplugClientMessageVariant::V3(msg) => msg.id(),
      ButtplugClientMessageVariant::V4(msg) => msg.id(),
    }
  }
  fn set_id(&mut self, id: u32) {
    match self {
      ButtplugClientMessageVariant::V0(msg) => msg.set_id(id),
      ButtplugClientMessageVariant::V1(msg) => msg.set_id(id),
      ButtplugClientMessageVariant::V2(msg) => msg.set_id(id),
      ButtplugClientMessageVariant::V3(msg) => msg.set_id(id),
      ButtplugClientMessageVariant::V4(msg) => msg.set_id(id),
    }
  }
}

impl ButtplugMessageFinalizer for ButtplugClientMessageVariant {
}

impl ButtplugMessageValidator for ButtplugClientMessageVariant {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugClientMessageVariant::V0(msg) => msg.is_valid(),
      ButtplugClientMessageVariant::V1(msg) => msg.is_valid(),
      ButtplugClientMessageVariant::V2(msg) => msg.is_valid(),
      ButtplugClientMessageVariant::V3(msg) => msg.is_valid(),
      ButtplugClientMessageVariant::V4(msg) => msg.is_valid(),
    }
  }
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
        ButtplugClientMessageV0::SingleMotorVibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV0::VorzeA10CycloneCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V1(msg) => match msg {
        ButtplugClientMessageV1::FleshlightLaunchFW12Cmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::SingleMotorVibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::VorzeA10CycloneCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV1::VibrateCmd(a) => Some(a.device_index()),
        _ => None,
      },
      Self::V2(msg) => match msg {
        ButtplugClientMessageV2::VibrateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::RotateCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::LinearCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV2::BatteryLevelCmd(a) => Some(a.device_index()),
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
        _ => None,
      },
      Self::V4(msg) => match msg {
        ButtplugClientMessageV4::OutputCmd(a) => Some(a.device_index()),
        ButtplugClientMessageV4::InputCmd(a) => Some(a.device_index()),
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

#[derive(Debug, Clone)]
pub enum ButtplugServerMessageVariant {
  V0(ButtplugServerMessageV0),
  V1(ButtplugServerMessageV1),
  V2(ButtplugServerMessageV2),
  V3(ButtplugServerMessageV3),
  V4(ButtplugServerMessageV4),
}

impl ButtplugMessage for ButtplugServerMessageVariant {
  fn id(&self) -> u32 {
    match self {
      ButtplugServerMessageVariant::V0(msg) => msg.id(),
      ButtplugServerMessageVariant::V1(msg) => msg.id(),
      ButtplugServerMessageVariant::V2(msg) => msg.id(),
      ButtplugServerMessageVariant::V3(msg) => msg.id(),
      ButtplugServerMessageVariant::V4(msg) => msg.id(),
    }
  }
  fn set_id(&mut self, id: u32) {
    match self {
      ButtplugServerMessageVariant::V0(msg) => msg.set_id(id),
      ButtplugServerMessageVariant::V1(msg) => msg.set_id(id),
      ButtplugServerMessageVariant::V2(msg) => msg.set_id(id),
      ButtplugServerMessageVariant::V3(msg) => msg.set_id(id),
      ButtplugServerMessageVariant::V4(msg) => msg.set_id(id),
    }
  }
}

impl ButtplugMessageFinalizer for ButtplugServerMessageVariant {
}

impl ButtplugMessageValidator for ButtplugServerMessageVariant {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugServerMessageVariant::V0(msg) => msg.is_valid(),
      ButtplugServerMessageVariant::V1(msg) => msg.is_valid(),
      ButtplugServerMessageVariant::V2(msg) => msg.is_valid(),
      ButtplugServerMessageVariant::V3(msg) => msg.is_valid(),
      ButtplugServerMessageVariant::V4(msg) => msg.is_valid(),
    }
  }
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
#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub enum ButtplugServerDeviceMessage {
  // Generic Sensor Reading Messages
  SensorReading(InputReadingV4),
}

impl ButtplugMessage for ButtplugServerDeviceMessage {
  fn id(&self) -> u32 {
    match self {
      ButtplugServerDeviceMessage::SensorReading(msg) => msg.id(),
    }
  }
  fn set_id(&mut self, id: u32) {
    match self {
      ButtplugServerDeviceMessage::SensorReading(msg) => msg.set_id(id),
    }
  }
}

impl ButtplugMessageFinalizer for ButtplugServerDeviceMessage {
}

impl ButtplugMessageValidator for ButtplugServerDeviceMessage {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugServerDeviceMessage::SensorReading(msg) => msg.is_valid(),
    }
  }
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
