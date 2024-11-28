// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  self, find_device_features, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, FeatureType, LegacyDeviceAttributes, RotateCmdV1, ScalarCmdV3, SingleMotorVibrateCmdV0, TryFromDeviceAttributes, VibrateCmdV1, VorzeA10CycloneCmdV0
};
use getset::{CopyGetters, Getters, MutGetters, Setters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generic command for setting a level (single magnitude value) of a device feature.
#[derive(Debug, PartialEq, Clone, CopyGetters, Setters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct LevelSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalar"))]
  level: i32,
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  #[getset(set = "pub")]
  feature_id: Option<Uuid>
}

impl LevelSubcommandV4 {
  pub fn new(feature_index: u32, level: i32, feature_id: &Option<Uuid>) -> Self {
    Self {
      feature_index,
      level,
      feature_id: feature_id.clone()
    }
  }
}

#[derive(
  Debug, Default, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters, MutGetters
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LevelCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Scalars"))]
  #[getset(get = "pub", get_mut = "pub(crate)")]
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

impl TryFromDeviceAttributes<VorzeA10CycloneCmdV0> for LevelCmdV4 {
  fn try_from_device_attributes(msg: VorzeA10CycloneCmdV0, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<LevelSubcommandV4> = features
    .features()
    .iter()
    .filter(|feature| *feature.feature_type() == FeatureType::RotateWithDirection)
    .map(|feature| {
      LevelSubcommandV4::new(
        0,
        (((msg.speed() as f64 / 99f64).ceil() * (if msg.clockwise() { 1f64 } else { -1f64 })) * *feature.actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32,
        &Some(feature.id().clone())
      )
    })
    .collect();

  Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceAttributes<SingleMotorVibrateCmdV0> for LevelCmdV4 {
  // For VibrateCmd, just take everything out of V2's VibrateCmd and make a command.
  fn try_from_device_attributes(msg: SingleMotorVibrateCmdV0, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<LevelSubcommandV4> = features
      .features()
      .iter()
      .filter(|feature| *feature.feature_type() == FeatureType::Vibrate)
      .map(|feature| {
        LevelSubcommandV4::new(
          0,
          (msg.speed() * *feature.actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32,
          &Some(feature.id().clone())
        )
      })
      .collect();
  
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceAttributes<VibrateCmdV1> for LevelCmdV4 {
  // VibrateCmd only exists up through Message Spec v2. We can assume that, if we're receiving it,
  // we can just use the V2 spec client device attributes for it. If this was sent on a V1 protocol,
  // it'll still have all the same features.
  fn try_from_device_attributes(msg: VibrateCmdV1, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let filtered_features = find_device_features(features.features(), |x| {
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
        0,
        (x.speed() * *filtered_features[x.index() as usize].actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32,
        &Some(filtered_features[x.index() as usize].id().clone())
      )
    })
    .collect();

    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceAttributes<ScalarCmdV3> for LevelCmdV4 {
  // ScalarCmd only came in with V3, so we can just use the V3 device attributes.
  fn try_from_device_attributes(msg: ScalarCmdV3, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<LevelSubcommandV4> = vec!(); 
    for cmd in msg.scalars() {
      // TODO this should be checked
      let feature = features.attrs_v3().scalar_cmd().as_ref().unwrap()[cmd.index() as usize].feature();
      cmds.push(LevelSubcommandV4::new(0, (cmd.scalar() * *feature.actuator().as_ref().unwrap().step_range().end() as f64).ceil() as i32, &Some(feature.id().clone())));
    }
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}

impl TryFromDeviceAttributes<RotateCmdV1> for LevelCmdV4 {
  // RotateCmd exists up through Message Spec v3. We can assume that, if we're receiving it, we can
  // just use the V3 spec client device attributes for it. If this was sent on a V1/V2 protocol,
  // it'll still have all the same features.
  fn try_from_device_attributes(msg: RotateCmdV1, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<LevelSubcommandV4> = vec!(); 
    for cmd in msg.rotations() {
      // TODO this should be checked
      let feature = features.attrs_v3().rotate_cmd().as_ref().unwrap()[cmd.index() as usize].feature();
      cmds.push(LevelSubcommandV4::new(0, (cmd.speed() * *feature.actuator().as_ref().unwrap().step_range().end() as f64 * (if cmd.clockwise() { 1f64 } else { -1f64 })).ceil() as i32, &Some(feature.id().clone())));
    }
    Ok(LevelCmdV4::new(msg.device_index(), cmds).into())
  }
}