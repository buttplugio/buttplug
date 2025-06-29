// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{ServerDeviceAttributes, TryFromDeviceAttributes};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    OutputCmdV4,
    OutputCommand,
  },
};

use getset::{CopyGetters, Getters};
use uuid::Uuid;

use super::spec_enums::ButtplugDeviceMessageNameV4;

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, Clone, Getters, CopyGetters, Eq,
)]
#[getset(get_copy = "pub")]
pub struct CheckedOutputCmdV4 {
  id: u32,
  device_index: u32,
  feature_index: u32,
  feature_id: Uuid,
  output_command: OutputCommand,
}

impl PartialEq for CheckedOutputCmdV4 {
  fn eq(&self, other: &Self) -> bool {
    // Compare everything but the message id
    self.device_index() == other.device_index()
      && self.feature_index() == other.feature_index()
      && self.feature_id() == other.feature_id()
      && self.output_command() == other.output_command()
  }
}

/*

impl From<CheckedActuatorCmdV4> for ActuatorCmdV4 {
  fn from(value: CheckedActuatorCmdV4) -> Self {
    ActuatorCmdV4::new(
      value.device_index(),
      value.feature_index(),
      value.actuator_type(),
      value.output_command()
    )
  }
}
  */

impl CheckedOutputCmdV4 {
  pub fn new(
    id: u32,
    device_index: u32,
    feature_index: u32,
    feature_id: Uuid,
    output_command: OutputCommand,
  ) -> Self {
    Self {
      id,
      device_index,
      feature_index,
      feature_id,
      output_command: output_command,
    }
  }
}

impl ButtplugMessageValidator for CheckedOutputCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<OutputCmdV4> for CheckedOutputCmdV4 {
  fn try_from_device_attributes(
    cmd: OutputCmdV4,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, ButtplugError> {
    let features = attrs.features();

    // Since we have the feature info already, check limit and unpack into step range when creating.
    //
    // If this message isn't the result of an upgrade from another older message, we won't have set
    // our feature id yet.
    let (feature, _) = if let Some(feature) = features.get(cmd.feature_index() as usize) {
      (feature, feature.id())
    } else {
      return Err(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureIndexError(features.len() as u32, cmd.feature_index()),
      ));
    };

    // Check to make sure the feature has an actuator that handles the data we've been passed
    if let Some(output_map) = feature.output() {
      if let Some(actuator) = output_map.get(&cmd.command().as_output_type()) {
        let value = cmd.command().value();
        let step_count = actuator.step_count();
        if value > step_count {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceStepRangeError(step_count, value),
          ))
        } else {
          // Only set adjusted value if we haven't gotten zero, otherwise assume stop.
          let new_value = if step_count != 0 && value != 0 {
            actuator.step_limit().start() + value
          } else {
            0
          };
          let mut new_command = cmd.command();
          new_command.set_value(new_value);
          // We can't make a private trait impl to turn a ValueCmd into a CheckedValueCmd, and this
          // is all about security, so we just copy. Silly, but it works for our needs in terms of
          // making this a barrier.
          Ok(Self {
            id: cmd.id(),
            feature_id: feature.id(),
            device_index: cmd.device_index(),
            feature_index: cmd.feature_index(),
            output_command: new_command,
          })
        }
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageNameV4::OutputCmd.to_string(),
          ),
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported(
          ButtplugDeviceMessageNameV4::OutputCmd.to_string(),
        ),
      ))
    }
  }
}
