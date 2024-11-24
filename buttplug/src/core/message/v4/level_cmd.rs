// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{errors::{ButtplugDeviceError, ButtplugError}, message::{
  self, find_device_feature_indexes, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceFeature, FeatureType, RotateCmdV1, ScalarCmdV3, SingleMotorVibrateCmdV0, TryFromDeviceFeatures, VibrateCmdV1, VorzeA10CycloneCmdV0
}};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct LevelSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  level: i32
}

impl LevelSubcommandV4 {
  pub fn new(feature_index: u32, level: i32) -> Self {
    Self {
      feature_index,
      level,
    }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LevelCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub")]
  levels: Vec<LevelSubcommandV4>,
}

impl LevelCmdV4 {
  pub fn new(device_index: u32, levels: Vec<LevelSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      levels,
    }
  }
}

impl ButtplugMessageValidator for LevelCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceFeatures<VorzeA10CycloneCmdV0> for LevelCmdV4 {
  fn try_from_device_features(msg: VorzeA10CycloneCmdV0, features: &[DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    let rotate_features: Vec<usize> = find_device_feature_indexes(features, |(_, x)| {
      *x.feature_type() == FeatureType::Rotate
        && x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::RotateCmd)
        })
    })?;

    let cmds: Vec<LevelSubcommandV4> = rotate_features
      .iter()
      .map(|x| {
        LevelSubcommandV4::new(
          *x as u32,
          ((msg.speed() as f64 / 99f64).ceil() * (if msg.clockwise() { 1f64 } else { -1f64 })) as i32,
        )
      })
      .collect();
  
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceFeatures<SingleMotorVibrateCmdV0> for LevelCmdV4 {
  fn try_from_device_features(msg: SingleMotorVibrateCmdV0, features: &[DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    let feature_indexes: Vec<usize> = find_device_feature_indexes(features, |(_, x)| {
      *x.feature_type() == FeatureType::Vibrate
        && x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::LevelCmd)
        })
    })?;

    let cmds: Vec<LevelSubcommandV4> = feature_indexes
      .iter()
      .map(|x| {
        LevelSubcommandV4::new(
          *x as u32,
          (msg.speed() * *features[*x].actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32,
        )
      })
      .collect();
  
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceFeatures<VibrateCmdV1> for LevelCmdV4 {
  fn try_from_device_features(msg: VibrateCmdV1, features: &[DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    let feature_indexes: Vec<usize> = find_device_feature_indexes(features, |(_, x)| {
      *x.feature_type() == FeatureType::Vibrate
        && x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::LevelCmd)
        })
    })?;

    let cmds: Vec<LevelSubcommandV4> = msg
    .speeds()
    .iter()
    .map(|x| {
      LevelSubcommandV4::new(
        feature_indexes[x.index() as usize] as u32,
        (x.speed() * *features[feature_indexes[x.index() as usize]].actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32,
      )
    })
    .collect();

    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceFeatures<ScalarCmdV3> for LevelCmdV4 {
  fn try_from_device_features(msg: ScalarCmdV3, features: &[DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    // We can assume here that ScalarCmd will translate directly to LevelCmd.
    let mut cmds: Vec<LevelSubcommandV4> = vec!(); 
    for cmd in msg.scalars() {
      cmds.push(LevelSubcommandV4::new(cmd.index(), (cmd.scalar() * *features.get(cmd.index() as usize).ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureIndexError(cmd.index(), features.len() as u32)))?.actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32));
    }
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceFeatures<RotateCmdV1> for LevelCmdV4 {
  fn try_from_device_features(msg: RotateCmdV1, features: &[DeviceFeature]) -> Result<Self, crate::core::errors::ButtplugError> {
    // We can assume here that ScalarCmd will translate directly to LevelCmd.
    let mut cmds: Vec<LevelSubcommandV4> = vec!(); 
    for cmd in msg.rotations() {
      cmds.push(LevelSubcommandV4::new(cmd.index(), (cmd.speed() * *features.get(cmd.index() as usize).ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureIndexError(cmd.index(), features.len() as u32)))?.actuator().as_ref().unwrap().step_range().end() as f64 * (if cmd.clockwise() { 1f64 } else { -1f64 })).ceil() as i32));
    }
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}