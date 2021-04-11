use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError},
  messages::{
    ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessageType, DeviceMessageAttributesMap,
    LinearCmd, RotateCmd, RotationSubcommand, VibrateCmd, VibrateSubcommand,
  },
};

pub struct GenericCommandManager {
  sent_vibration: bool,
  sent_rotation: bool,
  _sent_linear: bool,
  vibrations: Vec<u32>,
  vibration_step_counts: Vec<u32>,
  rotations: Vec<(u32, bool)>,
  rotation_step_counts: Vec<u32>,
  _linears: Vec<(u32, u32)>,
  _linear_step_counts: Vec<u32>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl GenericCommandManager {
  pub fn new(attributes: &DeviceMessageAttributesMap) -> Self {
    let mut vibrations: Vec<u32> = vec![];
    let mut vibration_step_counts: Vec<u32> = vec![];
    let mut rotations: Vec<(u32, bool)> = vec![];
    let mut rotation_step_counts: Vec<u32> = vec![];
    let mut linears: Vec<(u32, u32)> = vec![];
    let mut linear_step_counts: Vec<u32> = vec![];

    let mut stop_commands = vec![];

    // TODO We should probably panic here if we don't have feature and step counts?
    if let Some(attr) = attributes.get(&ButtplugDeviceMessageType::VibrateCmd) {
      if let Some(count) = attr.feature_count {
        vibrations = vec![0; count as usize];
      }
      if let Some(step_counts) = &attr.step_count {
        vibration_step_counts = step_counts.clone();
      }

      let mut subcommands = vec![];
      for i in 0..vibrations.len() {
        subcommands.push(VibrateSubcommand::new(i as u32, 0.0));
      }
      stop_commands.push(VibrateCmd::new(0, subcommands).into());
    }
    if let Some(attr) = attributes.get(&ButtplugDeviceMessageType::RotateCmd) {
      if let Some(count) = attr.feature_count {
        rotations = vec![(0, true); count as usize];
      }
      if let Some(step_counts) = &attr.step_count {
        rotation_step_counts = step_counts.clone();
      }

      // TODO Can we assume clockwise is false here? We might send extra
      // messages on Lovense since it'll require both a speed and change
      // direction command, but is that really a big deal? We can just
      // have it ignore the direction difference on a 0.0 speed?
      let mut subcommands = vec![];
      for i in 0..rotations.len() {
        subcommands.push(RotationSubcommand::new(i as u32, 0.0, false));
      }
      stop_commands.push(RotateCmd::new(0, subcommands).into());
    }
    if let Some(attr) = attributes.get(&ButtplugDeviceMessageType::LinearCmd) {
      if let Some(count) = attr.feature_count {
        linears = vec![(0, 0); count as usize];
      }
      if let Some(step_counts) = &attr.step_count {
        linear_step_counts = step_counts.clone();
      }
    }

    Self {
      sent_vibration: false,
      sent_rotation: false,
      _sent_linear: false,
      vibrations,
      rotations,
      _linears: linears,
      vibration_step_counts,
      rotation_step_counts,
      _linear_step_counts: linear_step_counts,
      stop_commands,
    }
  }

  pub fn update_vibration(
    &mut self,
    msg: &VibrateCmd,
    match_all: bool,
  ) -> Result<Option<Vec<Option<u32>>>, ButtplugError> {
    // First, make sure this is a valid command, that contains at least one
    // subcommand.
    if msg.speeds().is_empty() {
      return Err(
        ButtplugDeviceError::ProtocolRequirementError(
          "VibrateCmd has 0 commands, will not do anything.".to_owned(),
        )
        .into(),
      );
    }

    // Now we convert from the generic 0.0-1.0 range to the StepCount
    // attribute given by the device config.

    // If we've already sent commands before, we should check against our
    // old values. Otherwise, we should always send whatever command we're
    // going to send.
    let mut changed_value = false;
    let mut result: Vec<Option<u32>> = vec![None; self.vibrations.len()];
    // If we're in a match all situation, set up the array with all prior
    // values before switching them out.
    if match_all {
      for (index, speed) in self.vibrations.iter().enumerate() {
        result[index] = Some(*speed);
      }
    }
    for speed_command in msg.speeds() {
      let index = speed_command.index() as usize;
      // Since we're going to iterate here anyways, we do our index check
      // here instead of in a filter above.
      if index >= self.vibrations.len() {
        return Err(
          ButtplugDeviceError::ProtocolRequirementError(format!(
            "VibrateCmd has {} commands, device has {} vibrators.",
            msg.speeds().len(),
            self.vibrations.len()
          ))
          .into(),
        );
      }

      // When calculating speeds, round up. This follows how we calculated
      // things in buttplug-js and buttplug-csharp, so it's more for history
      // than anything, but it's what users will expect.
      let speed = (speed_command.speed() * self.vibration_step_counts[index] as f64).ceil() as u32;

      // If we've already sent commands, we don't want to send them again,
      // because some of our communication busses are REALLY slow. Make sure
      // these values get None in our return vector.
      if !self.sent_vibration || speed != self.vibrations[index] || match_all {
        // For some hardware, we always have to send all vibration
        // values, otherwise if we update one motor but not the other,
        // we'll stop the other motor completely if we send 0 to it.
        // That's what "match_all" is used for, so we always fall
        // through and set all of our values. However, in the case where
        // *no* motor speed changed, we don't want to send anything.
        // This is what changed_value checks.
        if speed != self.vibrations[index] || !self.sent_vibration {
          changed_value = true;
        }
        self.vibrations[index] = speed;
        result[index] = Some(speed);
      }
    }

    self.sent_vibration = true;

    // Return the command vector for the protocol to turn into proprietary commands
    if !changed_value {
      Ok(None)
    } else {
      Ok(Some(result))
    }
  }

  pub fn update_rotation(
    &mut self,
    msg: &RotateCmd,
  ) -> Result<Vec<Option<(u32, bool)>>, ButtplugError> {
    // First, make sure this is a valid command, that contains at least one
    // command.
    if msg.rotations.is_empty() {
      return Err(
        ButtplugDeviceError::ProtocolRequirementError(
          "RotateCmd has 0 commands, will not do anything.".to_owned(),
        )
        .into(),
      );
    }

    // Now we convert from the generic 0.0-1.0 range to the StepCount
    // attribute given by the device config.

    // If we've already sent commands before, we should check against our
    // old values. Otherwise, we should always send whatever command we're
    // going to send.
    let mut result: Vec<Option<(u32, bool)>> = vec![None; self.rotations.len()];
    for rotate_command in &msg.rotations {
      let index = rotate_command.index() as usize;
      // Since we're going to iterate here anyways, we do our index check
      // here instead of in a filter above.
      if index >= self.rotations.len() {
        return Err(
          ButtplugDeviceError::ProtocolRequirementError(format!(
            "RotateCmd has {} commands, device has {} rotators.",
            msg.rotations.len(),
            self.rotations.len()
          ))
          .into(),
        );
      }

      // When calculating speeds, round up. This follows how we calculated
      // things in buttplug-js and buttplug-csharp, so it's more for history
      // than anything, but it's what users will expect.
      let speed = (rotate_command.speed() * self.rotation_step_counts[index] as f64).ceil() as u32;
      let clockwise = rotate_command.clockwise();
      // If we've already sent commands, we don't want to send them again,
      // because some of our communication busses are REALLY slow. Make sure
      // these values get None in our return vector.
      if !self.sent_rotation
        || speed != self.rotations[index].0
        || clockwise != self.rotations[index].1
      {
        self.rotations[index] = (speed, clockwise);
        result[index] = Some((speed, clockwise));
      }
    }

    self.sent_rotation = true;

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn _update_linear(
    &mut self,
    _msg: &LinearCmd,
  ) -> Result<Option<Vec<(u32, u32)>>, ButtplugError> {
    // First, make sure this is a valid command, that doesn't contain an
    // index we can't reach.

    // If we've already sent commands before, we should check against our
    // old values. Otherwise, we should always send whatever command we're
    // going to send.

    // Now we convert from the generic 0.0-1.0 range to the StepCount
    // attribute given by the device config.

    // If we've already sent commands, we don't want to send them again,
    // because some of our communication busses are REALLY slow. Make sure
    // these values get None in our return vector.

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(None)
  }

  pub fn get_stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
    self.stop_commands.clone()
  }
}

#[cfg(test)]
mod test {

  use super::GenericCommandManager;
  use crate::core::messages::{
    ButtplugDeviceMessageType, DeviceMessageAttributes, DeviceMessageAttributesMap, RotateCmd,
    RotationSubcommand, VibrateCmd, VibrateSubcommand,
  };
  #[test]
  pub fn test_command_generator_vibration() {
    let mut attributes_map = DeviceMessageAttributesMap::new();

    let vibrate_attributes = DeviceMessageAttributes {
      feature_count: Some(2),
      step_count: Some(vec![20, 20]),
      ..Default::default()
    };
    attributes_map.insert(ButtplugDeviceMessageType::VibrateCmd, vibrate_attributes);
    let mut mgr = GenericCommandManager::new(&attributes_map);
    let vibrate_msg = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.5),
      ],
    );
    assert_eq!(
      mgr.update_vibration(&vibrate_msg, false).unwrap(),
      Some(vec![Some(10), Some(10)])
    );
    assert_eq!(mgr.update_vibration(&vibrate_msg, false).unwrap(), None);
    let vibrate_msg_2 = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.75),
      ],
    );
    assert_eq!(
      mgr.update_vibration(&vibrate_msg_2, false).unwrap(),
      Some(vec![None, Some(15)])
    );
    let vibrate_msg_invalid = VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 0.5)]);
    assert!(mgr.update_vibration(&vibrate_msg_invalid, false).is_err());
  }

  #[test]
  pub fn test_command_generator_rotation() {
    let mut attributes_map = DeviceMessageAttributesMap::new();

    let rotate_attributes = DeviceMessageAttributes {
      feature_count: Some(2),
      step_count: Some(vec![20, 20]),
      ..Default::default()
    };
    attributes_map.insert(ButtplugDeviceMessageType::RotateCmd, rotate_attributes);
    let mut mgr = GenericCommandManager::new(&attributes_map);
    let rotate_msg = RotateCmd::new(
      0,
      vec![
        RotationSubcommand::new(0, 0.5, true),
        RotationSubcommand::new(1, 0.5, true),
      ],
    );
    assert_eq!(
      mgr.update_rotation(&rotate_msg).unwrap(),
      vec![Some((10, true)), Some((10, true))]
    );
    assert_eq!(mgr.update_rotation(&rotate_msg).unwrap(), vec![None, None]);
    let rotate_msg_2 = RotateCmd::new(
      0,
      vec![
        RotationSubcommand::new(0, 0.5, true),
        RotationSubcommand::new(1, 0.75, false),
      ],
    );
    assert_eq!(
      mgr.update_rotation(&rotate_msg_2).unwrap(),
      vec![None, Some((15, false))]
    );
    let rotate_msg_invalid = RotateCmd::new(0, vec![RotationSubcommand::new(2, 0.5, true)]);
    assert!(mgr.update_rotation(&rotate_msg_invalid).is_err());
  }

  // TODO Write test for vibration stop generator
}
