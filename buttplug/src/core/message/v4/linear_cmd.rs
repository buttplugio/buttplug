// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  self, find_device_feature_indexes, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, LinearCmdV1, TryFromDeviceFeatures
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Move device to a certain position in a certain amount of time
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct VectorSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: f64,
}

impl VectorSubcommandV4 {
  pub fn new(feature_index: u32, duration: u32, position: f64) -> Self {
    Self {
      feature_index,
      duration,
      position,
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LinearCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  #[getset(get = "pub")]
  vectors: Vec<VectorSubcommandV4>,
}

impl LinearCmdV4 {
  pub fn new(device_index: u32, vectors: Vec<VectorSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }
}

impl ButtplugMessageValidator for LinearCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceFeatures<LinearCmdV1> for LinearCmdV4 {
  fn try_from_device_features(msg: LinearCmdV1, features: &[crate::core::message::DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    let linear_features: Vec<usize> =
      find_device_feature_indexes(features, |(_, x)| {
        x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::LinearCmd)
        })
      })?;

    let cmds: Vec<VectorSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        VectorSubcommandV4::new(
          linear_features[x.index() as usize] as u32,
          x.duration(),
          x.position(),
        )
      })
      .collect();

    Ok(LinearCmdV4::new(msg.device_index(), cmds).into())
  }
}