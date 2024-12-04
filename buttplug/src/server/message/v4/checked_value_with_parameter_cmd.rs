use crate::{
  core::{
    errors::ButtplugMessageError,
    message::{
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      ValueWithParameterCmdV4,
      ValueWithParameterSubcommandV4,
    },
  },
  server::message::{v1::LinearCmdV1, ServerDeviceAttributes, TryFromDeviceAttributes},
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Move device to a certain position in a certain amount of time
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct CheckedValueWithParameterSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: f64,
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  id: Uuid,
}

impl CheckedValueWithParameterSubcommandV4 {
  pub fn new(feature_index: u32, duration: u32, position: f64, id: Uuid) -> Self {
    Self {
      feature_index,
      duration,
      position,
      id,
    }
  }
}

impl From<CheckedValueWithParameterSubcommandV4> for ValueWithParameterSubcommandV4 {
  fn from(value: CheckedValueWithParameterSubcommandV4) -> Self {
    Self::new(value.feature_index(), value.duration(), value.position())
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct CheckedValueWithParameterCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  #[getset(get = "pub")]
  vectors: Vec<CheckedValueWithParameterSubcommandV4>,
}

impl CheckedValueWithParameterCmdV4 {
  pub fn new(device_index: u32, vectors: Vec<CheckedValueWithParameterSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }
}

impl From<CheckedValueWithParameterCmdV4> for ValueWithParameterCmdV4 {
  fn from(value: CheckedValueWithParameterCmdV4) -> Self {
    Self::new(
      value.device_index(),
      value
        .vectors()
        .iter()
        .map(|x| ValueWithParameterSubcommandV4::from(x.clone()))
        .collect(),
    )
  }
}

impl ButtplugMessageValidator for CheckedValueWithParameterCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<LinearCmdV1> for CheckedValueWithParameterCmdV4 {
  fn try_from_device_attributes(
    msg: LinearCmdV1,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<CheckedValueWithParameterSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        CheckedValueWithParameterSubcommandV4::new(
          0,
          x.duration(),
          x.position(),
          *features.attrs_v3().linear_cmd().as_ref().unwrap()[x.index() as usize]
            .feature()
            .id(),
        )
      })
      .collect();

    Ok(CheckedValueWithParameterCmdV4::new(msg.device_index(), cmds))
  }
}

impl TryFromDeviceAttributes<ValueWithParameterCmdV4> for CheckedValueWithParameterCmdV4 {
  fn try_from_device_attributes(
    msg: ValueWithParameterCmdV4,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<CheckedValueWithParameterSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        CheckedValueWithParameterSubcommandV4::new(
          0,
          x.duration(),
          x.position(),
          *features.features()[x.feature_index() as usize].id(),
        )
      })
      .collect();

    Ok(CheckedValueWithParameterCmdV4::new(msg.device_index(), cmds))
  }
}
