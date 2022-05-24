// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::{
      ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessageType, LinearCmd, RotateCmd,
      RotationSubcommand, VibrateCmd, VibrateSubcommand,
    },
  },
  server::device::configuration::ProtocolDeviceAttributes,
};
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering::SeqCst};

// In order to make our lives easier, we make some assumptions about what's internally mutable in
// the GenericCommandManager (GCM). Once the GCM is configured for a device, it won't change sizes,
// because we don't support things like adding motors to devices randomly while Buttplug is running.
// Therefore we know that we'll just be storing values like vibration/rotation speeds. We can assume
// our storage of those can stay immutable (the vec sizes won't change) and make their internals
// mutable. While this could be RefCell'd or whatever, they're also always atomic types (until the
// horrible day some sex toy decides to use floats in its protocol), so we can just use atomics and
// call it done.
pub struct GenericCommandManager {
  sent_vibration: AtomicBool,
  sent_rotation: AtomicBool,
  _sent_linear: bool,
  vibrations: Vec<AtomicU32>,
  vibration_step_ranges: Vec<(u32, u32)>,
  rotations: Vec<(AtomicU32, AtomicBool)>,
  rotation_step_ranges: Vec<(u32, u32)>,
  _linears: Vec<(u32, u32)>,
  _linear_step_counts: Vec<u32>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl GenericCommandManager {
  pub fn new(attributes: &ProtocolDeviceAttributes) -> Self {
    let mut vibrations = vec![];
    let mut vibration_step_counts = vec![];
    let mut vibration_step_ranges = vec![];
    let mut rotations = vec![];
    let mut rotation_step_counts = vec![];
    let mut rotation_step_ranges = vec![];
    let mut linears= vec![];
    let mut linear_step_counts = vec![];

    let mut stop_commands = vec![];

    // TODO We should probably panic here if we don't have feature and step counts?
    if let Some(attr) = attributes.message_attributes(&ButtplugDeviceMessageType::VibrateCmd) {
      if let Some(count) = attr.feature_count() {
        // We have to use resize_with here, since Atomic* aren't clonable.
        vibrations.resize_with(*count as usize, || AtomicU32::new(0));
      }
      if let Some(step_counts) = &attr.step_count() {
        vibration_step_counts = step_counts.clone();
      }
      if let Some(step_range) = &attr.step_range() {
        vibration_step_ranges = step_range.clone();
      } else {
        for step_count in &vibration_step_counts {
          vibration_step_ranges.push((0, *step_count));
        }
      }

      let mut subcommands = vec![];
      for i in 0..vibrations.len() {
        subcommands.push(VibrateSubcommand::new(i as u32, 0.0));
      }
      stop_commands.push(VibrateCmd::new(0, subcommands).into());
    }
    if let Some(attr) = attributes.message_attributes(&ButtplugDeviceMessageType::RotateCmd) {
      if let Some(count) = attr.feature_count() {
        // We have to use resize_with here, since Atomic* aren't clonable.
        rotations.resize_with(*count as usize, || (AtomicU32::new(0), AtomicBool::new(false)));
      }
      if let Some(step_counts) = &attr.step_count() {
        rotation_step_counts = step_counts.clone();
      }
      if let Some(step_range) = &attr.step_range() {
        rotation_step_ranges = step_range.clone();
      } else {
        for step_count in &rotation_step_counts {
          rotation_step_ranges.push((0, *step_count));
        }
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
    if let Some(attr) = attributes.message_attributes(&ButtplugDeviceMessageType::LinearCmd) {
      if let Some(count) = attr.feature_count() {
        linears = vec![(0, 0); *count as usize];
      }
      if let Some(step_counts) = &attr.step_count() {
        linear_step_counts = step_counts.clone();
      }
    }

    Self {
      sent_vibration: AtomicBool::new(false),
      sent_rotation: AtomicBool::new(false),
      _sent_linear: false,
      vibrations,
      rotations,
      _linears: linears,
      vibration_step_ranges,
      rotation_step_ranges,
      _linear_step_counts: linear_step_counts,
      stop_commands,
    }
  }

  pub fn update_vibration(
    &self,
    msg: &VibrateCmd,
    match_all: bool,
  ) -> Result<Vec<Option<u32>>, ButtplugError> {
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
    let mut result: Vec<Option<u32>> = vec![None; self.vibrations.len()];
    // If we're in a match all situation, set up the array with all prior
    // values before switching them out.
    if match_all {
      for (index, speed) in self.vibrations.iter().enumerate() {
        result[index] = Some(speed.load(SeqCst));
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

      let range = self.vibration_step_ranges[index].1 - self.vibration_step_ranges[index].0;
      let speed_modifier = speed_command.speed() * range as f64;
      let speed = if speed_modifier < 0.0001 {
        0
      } else {
        // When calculating speeds, round up. This follows how we calculated
        // things in buttplug-js and buttplug-csharp, so it's more for history
        // than anything, but it's what users will expect.
        (speed_modifier + self.vibration_step_ranges[index].0 as f64).ceil() as u32
      };
      info!(
        "{:?} {} {} {}",
        self.vibration_step_ranges[index], range, speed_modifier, speed
      );
      // If we've already sent commands, we don't want to send them again,
      // because some of our communication busses are REALLY slow. Make sure
      // these values get None in our return vector.
      let current_speed = self.vibrations[index].load(SeqCst);
      let sent_vibration = self.sent_vibration.load(SeqCst);
      if !sent_vibration || speed != current_speed || match_all {
        self.vibrations[index].store(speed, SeqCst);
        result[index] = Some(speed);
      }

      if !sent_vibration {
        self.sent_vibration.store(true, SeqCst);
      }
    }

    // If we have no changes to the device, just send back an empty command array. We have nothing
    // to do.
    if result.iter().all(|x| x.is_none()) {
      result.clear();
    }

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn vibration(&self) -> Vec<Option<u32>> {
    self.vibrations.iter().map(|x| Some(x.load(SeqCst))).collect()
  }

  pub fn update_rotation(
    &self,
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
      let range = self.rotation_step_ranges[index].1 - self.rotation_step_ranges[index].0;
      let speed_modifier = rotate_command.speed() * range as f64;
      let speed = if speed_modifier < 0.0001 {
        0
      } else {
        // When calculating speeds, round up. This follows how we calculated
        // things in buttplug-js and buttplug-csharp, so it's more for history
        // than anything, but it's what users will expect.
        (speed_modifier + self.rotation_step_ranges[index].0 as f64).ceil() as u32
      };
      let clockwise = rotate_command.clockwise();
      // If we've already sent commands, we don't want to send them again,
      // because some of our communication busses are REALLY slow. Make sure
      // these values get None in our return vector.
      let sent_rotation = self.sent_rotation.load(SeqCst);
      if !sent_rotation
        || speed != self.rotations[index].0.load(SeqCst)
        || clockwise != self.rotations[index].1.load(SeqCst)
      {
        self.rotations[index].0.store(speed, SeqCst);
        self.rotations[index].1.store(clockwise, SeqCst);
        result[index] = Some((speed, clockwise));
      }
      if !sent_rotation {
        self.sent_rotation.store(true, SeqCst);
      }
    }

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn _update_linear(
    &self,
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

  pub fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
    self.stop_commands.clone()
  }
}

#[cfg(test)]
mod test {

  use super::{GenericCommandManager, ProtocolDeviceAttributes};
  use crate::{
    core::messages::{
      ButtplugDeviceMessageType, DeviceMessageAttributesBuilder, DeviceMessageAttributesMap,
      RotateCmd, RotationSubcommand, VibrateCmd, VibrateSubcommand,
    },
    server::device::configuration::ProtocolAttributesType,
  };

  #[test]
  pub fn test_command_generator_vibration() {
    let mut attributes_map = DeviceMessageAttributesMap::new();

    let vibrate_attributes = DeviceMessageAttributesBuilder::default()
      .feature_count(2)
      .step_count(vec![20, 20])
      .build(&ButtplugDeviceMessageType::VibrateCmd)
      .unwrap();
    attributes_map.insert(ButtplugDeviceMessageType::VibrateCmd, vibrate_attributes);
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      attributes_map,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);
    let vibrate_msg = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.5),
      ],
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![Some(10), Some(10)]
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![]
    );
    let vibrate_msg_2 = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.75),
      ],
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg_2, false)
        .expect("Test, assuming infallible"),
      vec![None, Some(15)]
    );
    let vibrate_msg_invalid = VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 0.5)]);
    assert!(mgr.update_vibration(&vibrate_msg_invalid, false).is_err());

    assert_eq!(mgr.vibration(), vec![Some(10), Some(15)]);
  }

  #[test]
  pub fn test_command_generator_vibration_step_range() {
    let mut attributes_map = DeviceMessageAttributesMap::new();

    let vibrate_attributes = DeviceMessageAttributesBuilder::default()
      .feature_count(2)
      .step_count(vec![20, 20])
      .step_range(vec![(10, 15), (10, 20)])
      .build(&ButtplugDeviceMessageType::VibrateCmd)
      .unwrap();
    attributes_map.insert(ButtplugDeviceMessageType::VibrateCmd, vibrate_attributes);
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      attributes_map,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);
    let vibrate_msg = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.5),
      ],
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![Some(13), Some(15)]
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![]
    );
    let vibrate_msg_2 = VibrateCmd::new(
      0,
      vec![
        VibrateSubcommand::new(0, 0.5),
        VibrateSubcommand::new(1, 0.75),
      ],
    );
    assert_eq!(
      mgr
        .update_vibration(&vibrate_msg_2, false)
        .expect("Test, assuming infallible"),
      vec![None, Some(18)]
    );
    let vibrate_msg_invalid = VibrateCmd::new(0, vec![VibrateSubcommand::new(2, 0.5)]);
    assert!(mgr.update_vibration(&vibrate_msg_invalid, false).is_err());

    assert_eq!(mgr.vibration(), vec![Some(13), Some(18)]);
  }

  #[test]
  pub fn test_command_generator_rotation() {
    let mut attributes_map = DeviceMessageAttributesMap::new();

    let rotate_attributes = DeviceMessageAttributesBuilder::default()
      .feature_count(2)
      .step_count(vec![20, 20])
      .build(&ButtplugDeviceMessageType::RotateCmd)
      .unwrap();
    attributes_map.insert(ButtplugDeviceMessageType::RotateCmd, rotate_attributes);
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      attributes_map,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);

    let rotate_msg = RotateCmd::new(
      0,
      vec![
        RotationSubcommand::new(0, 0.5, true),
        RotationSubcommand::new(1, 0.5, true),
      ],
    );
    assert_eq!(
      mgr
        .update_rotation(&rotate_msg)
        .expect("Test, assuming infallible"),
      vec![Some((10, true)), Some((10, true))]
    );
    assert_eq!(
      mgr
        .update_rotation(&rotate_msg)
        .expect("Test, assuming infallible"),
      vec![None, None]
    );
    let rotate_msg_2 = RotateCmd::new(
      0,
      vec![
        RotationSubcommand::new(0, 0.5, true),
        RotationSubcommand::new(1, 0.75, false),
      ],
    );
    assert_eq!(
      mgr
        .update_rotation(&rotate_msg_2)
        .expect("Test, assuming infallible"),
      vec![None, Some((15, false))]
    );
    let rotate_msg_invalid = RotateCmd::new(0, vec![RotationSubcommand::new(2, 0.5, true)]);
    assert!(mgr.update_rotation(&rotate_msg_invalid).is_err());
  }
  // TODO Write test for vibration stop generator
}
