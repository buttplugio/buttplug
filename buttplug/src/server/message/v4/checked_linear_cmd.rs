use crate::{
  core::{
    errors::ButtplugMessageError,
    message::{
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      LinearCmdV4,
    },
  },
  server::message::{v1::LinearCmdV1, LegacyDeviceAttributes, TryFromDeviceAttributes},
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Move device to a certain position in a certain amount of time
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct CheckedVectorSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: f64,
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  id: Uuid,
}

impl CheckedVectorSubcommandV4 {
  pub fn new(feature_index: u32, duration: u32, position: f64, id: Uuid) -> Self {
    Self {
      feature_index,
      duration,
      position,
      id,
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct CheckedLinearCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  #[getset(get = "pub")]
  vectors: Vec<CheckedVectorSubcommandV4>,
}

impl CheckedLinearCmdV4 {
  pub fn new(device_index: u32, vectors: Vec<CheckedVectorSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }
}

impl ButtplugMessageValidator for CheckedLinearCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<LinearCmdV1> for CheckedLinearCmdV4 {
  fn try_from_device_attributes(
    msg: LinearCmdV1,
    features: &LegacyDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<CheckedVectorSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        CheckedVectorSubcommandV4::new(
          0,
          x.duration(),
          x.position(),
          *features.attrs_v3().linear_cmd().as_ref().unwrap()[x.index() as usize]
            .feature()
            .id(),
        )
      })
      .collect();

    Ok(CheckedLinearCmdV4::new(msg.device_index(), cmds))
  }
}

impl TryFromDeviceAttributes<LinearCmdV4> for CheckedLinearCmdV4 {
  fn try_from_device_attributes(
    msg: LinearCmdV4,
    features: &LegacyDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<CheckedVectorSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        CheckedVectorSubcommandV4::new(
          0,
          x.duration(),
          x.position(),
          *features.features()[x.feature_index() as usize].id(),
        )
      })
      .collect();

    Ok(CheckedLinearCmdV4::new(msg.device_index(), cmds))
  }
}
