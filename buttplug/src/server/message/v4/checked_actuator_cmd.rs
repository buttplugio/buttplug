// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ActuatorCmdV4, ActuatorCommand, ActuatorType, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator,
    },
  },
  server::message::{
    ServerDeviceAttributes, TryFromDeviceAttributes, VorzeA10CycloneCmdV0
  },
};
use getset::{CopyGetters, Getters};
use uuid::Uuid;

use super::spec_enums::ButtplugDeviceMessageNameV4;

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  Clone,
  Getters,
  CopyGetters,
  Eq,
)]
#[getset(get_copy = "pub")]
pub struct CheckedActuatorCmdV4 {
  id: u32,
  device_index: u32,
  feature_index: u32,
  feature_id: Uuid,
  actuator_type: ActuatorType,
  actuator_command: ActuatorCommand
}

impl PartialEq for CheckedActuatorCmdV4 {
  fn eq(&self, other: &Self) -> bool {
    // Compare everything but the message id
    self.device_index() == other.device_index() &&
    self.feature_index() == other.feature_index() &&
    self.actuator_type() == other.actuator_type() &&
    self.feature_id() == other.feature_id() &&
    self.actuator_command() == other.actuator_command()
  }
}

impl From<CheckedActuatorCmdV4> for ActuatorCmdV4 {
  fn from(value: CheckedActuatorCmdV4) -> Self {
    ActuatorCmdV4::new(
      value.device_index(),
      value.feature_index(),
      value.actuator_type(),
      value.actuator_command()
    )
  }
}

impl CheckedActuatorCmdV4 {
  pub fn new(id: u32, device_index: u32, feature_index: u32, feature_id: Uuid, actuator_type: ActuatorType, actuator_command: ActuatorCommand) -> Self {
    Self {
      id,
      device_index,
      feature_index,
      feature_id,
      actuator_type,
      actuator_command
    }
  }
}

impl ButtplugMessageValidator for CheckedActuatorCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}


impl TryFromDeviceAttributes<ActuatorCmdV4> for CheckedActuatorCmdV4 {
  fn try_from_device_attributes(
    cmd: ActuatorCmdV4,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, ButtplugError> {
    let features = attrs.features();
    // Since we have the feature info already, check limit and unpack into step range when creating
    // If this message isn't the result of an upgrade from another older message, we won't have set our feature yet.
    let feature_id = if let Some(feature) = features.get(cmd.feature_index() as usize) {
      feature.id()
    } else {
      return Err(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureIndexError(
          features.len() as u32,
          cmd.feature_index(),
        ),
      ));
    };

    let feature = features
      .iter()
      .find(|x| x.id() == feature_id)
      .expect("Already checked existence or created.");
    let level = cmd.value();
    // Check to make sure the feature has an actuator that handles ValueCmd
    if let Some(actuator_map) = feature.actuator() {
      if let Some(actuator) = actuator_map.get(&cmd.actuator_type()) {
        // Check to make sure the level is within the range of the feature.
        if actuator
          .messages()
          .contains(&crate::core::message::ButtplugActuatorFeatureMessageType::ValueCmd)
        {
          if level > actuator.step_count() {
            Err(ButtplugError::from(
              ButtplugDeviceError::DeviceStepRangeError(
                *actuator.step_limit().end(),
                level,
              ),
            ))
          } else {
            // We can't make a private trait impl to turn a ValueCmd into a CheckedValueCmd, and this
            // is all about security, so we just copy. Silly, but it works for our needs in terms of
            // making this a barrier.
            Ok(Self {
              id: cmd.id(),
              feature_id: feature.id(),
              device_index: cmd.device_index(),
              feature_index: cmd.feature_index(),
              actuator_type: cmd.actuator_type(),
              actuator_command: cmd.actuator_command()
            })
          }
        } else {
          Err(ButtplugError::from(
            ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::ValueCmd.to_string()),
          ))
        }
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::ValueCmd.to_string()),
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageNameV4::ValueCmd.to_string()),
      ))
    }
  }
}

// Converting Vorze A10 Cyclone commands is difficult because we have to assume that the device
// we're converting for is anything like a Vorze A10 Cyclone. This would mean it has 1 directional
// rotating element. We currently don't have any devices with more than 1 rotating element, so this
// assumption works fine for now, but assuming we ever get to something that has 2 or more (and I
// could see this happening, like a stroker with independent shaft/head rotation), should this drive
// all of them the same way? Or just 1?
//
// For now, we're assuming it'll only run the first RotateWithDirection device found.
//
// And the bigger question is: Did anyone ever even use this message? We phased it out early, it may
// just not exist in the wild anymore. :P
impl TryFromDeviceAttributes<VorzeA10CycloneCmdV0> for CheckedActuatorCmdV4 {
  fn try_from_device_attributes(
    msg: VorzeA10CycloneCmdV0,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let features: Vec<(usize, &ServerDeviceFeature)> = features
      .features()
      .iter()
      .enumerate()
      .filter(|(_, feature)| feature.feature_type() == FeatureType::RotateWithDirection)
      .collect();
      
    if features.is_empty() {
      return Err(ButtplugError::from(ButtplugDeviceError::DeviceFeatureMismatch("Device has no RotateWithDirection features".to_owned())));
    }

    let feature = features[0];
    let actuator = feature.1
      .actuator()
      .as_ref()
      .ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureMismatch("RotationWithDirection feature has no actuator".to_owned())))?
      .get(&ActuatorType::RotateWithDirection)
      .ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureMismatch("RotationWithDirection feature has no actuator".to_owned())))?;
    
    Ok(CheckedValueWithParameterCmdV4::new(
      msg.device_index(),
      feature.0 as u32,
      feature.1.id(),
      ActuatorType::RotateWithDirection,
      ((msg.speed() as f64 / 99f64).ceil() * (((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil()) as u32,
      if msg.clockwise() { 1 } else { -1 }
    ))
  }
}