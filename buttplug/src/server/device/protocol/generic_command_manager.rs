// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    message::{
      ActuatorType,
      ButtplugDeviceCommandMessageUnion,
      LinearCmd,
      RotateCmd,
      RotationSubcommand,
      ScalarCmd,
      ScalarSubcommand,
    },
  },
  server::device::configuration::{ProtocolDeviceAttributes, ServerGenericDeviceMessageAttributes},
};
use getset::Getters;
use std::{
  ops::RangeInclusive,
  sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst},
};

#[derive(Getters)]
#[getset(get = "pub")]
struct ScalarGenericCommand {
  actuator: ActuatorType,
  step_range: RangeInclusive<u32>,
  value: AtomicU32,
}

impl ScalarGenericCommand {
  pub fn new(attributes: &ServerGenericDeviceMessageAttributes) -> Self {
    Self {
      actuator: *attributes.actuator_type(),
      step_range: attributes.step_range().clone(),
      value: AtomicU32::new(0),
    }
  }
}

// In order to make our lives easier, we make some assumptions about what's internally mutable in
// the GenericCommandManager (GCM). Once the GCM is configured for a device, it won't change sizes,
// because we don't support things like adding motors to devices randomly while Buttplug is running.
// Therefore we know that we'll just be storing values like vibration/rotation speeds. We can assume
// our storage of those can stay immutable (the vec sizes won't change) and make their internals
// mutable. While this could be RefCell'd or whatever, they're also always atomic types (until the
// horrible day some sex toy decides to use floats in its protocol), so we can just use atomics and
// call it done.
pub struct GenericCommandManager {
  sent_scalar: AtomicBool,
  sent_rotation: AtomicBool,
  _sent_linear: bool,
  scalars: Vec<ScalarGenericCommand>,
  rotations: Vec<(AtomicU32, AtomicBool)>,
  rotation_step_ranges: Vec<RangeInclusive<u32>>,
  _linears: Vec<(u32, u32)>,
  _linear_step_counts: Vec<u32>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl GenericCommandManager {
  pub fn new(attributes: &ProtocolDeviceAttributes) -> Self {
    let mut scalars = vec![];
    let mut rotations = vec![];
    let mut rotation_step_ranges = vec![];
    let mut linears = vec![];
    let mut linear_step_counts = vec![];

    let mut stop_commands = vec![];

    if let Some(attrs) = attributes.message_attributes.scalar_cmd() {
      let mut subcommands = vec![];
      for (index, attr) in attrs.iter().enumerate() {
        scalars.push(ScalarGenericCommand::new(attr));
        subcommands.push(ScalarSubcommand::new(
          index as u32,
          0.0,
          *attr.actuator_type(),
        ));
      }

      stop_commands.push(ScalarCmd::new(0, subcommands).into());
    }
    if let Some(attrs) = attributes.message_attributes.rotate_cmd() {
      rotations.resize_with(attrs.len(), || (AtomicU32::new(0), AtomicBool::new(false)));
      for attr in attrs {
        rotation_step_ranges.push(attr.step_range().clone());
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
    if let Some(attrs) = attributes.message_attributes.linear_cmd() {
      linears = vec![(0, 0); attrs.len()];
      for attr in attrs {
        linear_step_counts.push(attr.step_count());
      }
    }

    Self {
      sent_scalar: AtomicBool::new(false),
      sent_rotation: AtomicBool::new(false),
      _sent_linear: false,
      scalars,
      rotations,
      _linears: linears,
      rotation_step_ranges,
      _linear_step_counts: linear_step_counts,
      stop_commands,
    }
  }

  pub fn update_scalar(
    &self,
    msg: &ScalarCmd,
    match_all: bool,
  ) -> Result<Vec<Option<(ActuatorType, u32)>>, ButtplugError> {
    // First, make sure this is a valid command, that contains at least one
    // subcommand.
    if msg.scalars().is_empty() {
      return Err(
        ButtplugDeviceError::ProtocolRequirementError(
          "ScalarCmd has 0 commands, will not do anything.".to_owned(),
        )
        .into(),
      );
    }

    // Now we convert from the generic 0.0-1.0 range to the StepCount
    // attribute given by the device config.

    // If we've already sent commands before, we should check against our
    // old values. Otherwise, we should always send whatever command we're
    // going to send.
    let mut result: Vec<Option<(ActuatorType, u32)>> = vec![None; self.scalars.len()];

    for scalar_command in msg.scalars() {
      let index = scalar_command.index() as usize;
      // Since we're going to iterate here anyways, we do our index check
      // here instead of in a filter above.
      if index >= self.scalars.len() {
        return Err(
          ButtplugDeviceError::ProtocolRequirementError(format!(
            "ScalarCmd has {} commands, device has {} features.",
            msg.scalars().len(),
            self.scalars.len()
          ))
          .into(),
        );
      }

      let range_start = self.scalars[index].step_range().start();
      let range = self.scalars[index].step_range().end() - range_start;
      let scalar_modifier = scalar_command.scalar() * range as f64;
      let scalar = if scalar_modifier < 0.0001 {
        0
      } else {
        // When calculating speeds, round up. This follows how we calculated
        // things in buttplug-js and buttplug-csharp, so it's more for history
        // than anything, but it's what users will expect.
        (scalar_modifier + *range_start as f64).ceil() as u32
      };
      trace!(
        "{:?} {} {} {}",
        self.scalars[index].step_range(),
        range,
        scalar_modifier,
        scalar
      );
      // If we've already sent commands, we don't want to send them again,
      // because some of our communication busses are REALLY slow. Make sure
      // these values get None in our return vector.
      let current_scalar = self.scalars[index].value().load(SeqCst);
      let sent_scalar = self.sent_scalar.load(SeqCst);
      if !sent_scalar || scalar != current_scalar {
        self.scalars[index].value().store(scalar, SeqCst);
        result[index] = Some((*self.scalars[index].actuator(), scalar));
      }

      if !sent_scalar {
        self.sent_scalar.store(true, SeqCst);
      }
    }

    // If we have no changes to the device, just send back an empty command array. We have nothing
    // to do.
    if result.iter().all(|x| x.is_none()) {
      result.clear();
    } else if match_all {
      // If we're in a match all situation, set up the array with all prior
      // values before switching them out.
      for (index, cmd) in self.scalars.iter().enumerate() {
        if result[index].is_none() {
          result[index] = Some((*cmd.actuator(), cmd.value.load(SeqCst)));
        }
      }
    }

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  // Test method
  #[cfg(test)]
  pub(super) fn scalars(&self) -> Vec<Option<(ActuatorType, u32)>> {
    self
      .scalars
      .iter()
      .map(|x| Some((*x.actuator(), x.value().load(SeqCst))))
      .collect()
  }

  pub fn update_rotation(
    &self,
    msg: &RotateCmd,
    match_all: bool,
  ) -> Result<Vec<Option<(u32, bool)>>, ButtplugError> {
    // First, make sure this is a valid command, that contains at least one
    // command.
    if msg.rotations().is_empty() {
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
    for rotate_command in msg.rotations() {
      let index = rotate_command.index() as usize;
      // Since we're going to iterate here anyways, we do our index check
      // here instead of in a filter above.
      if index >= self.rotations.len() {
        return Err(
          ButtplugDeviceError::ProtocolRequirementError(format!(
            "RotateCmd has {} commands, device has {} rotators.",
            msg.rotations().len(),
            self.rotations.len()
          ))
          .into(),
        );
      }

      // When calculating speeds, round up. This follows how we calculated
      // things in buttplug-js and buttplug-csharp, so it's more for history
      // than anything, but it's what users will expect.
      let range = self.rotation_step_ranges[index].end() - self.rotation_step_ranges[index].start();
      let speed_modifier = rotate_command.speed() * range as f64;
      let speed = if speed_modifier < 0.0001 {
        0
      } else {
        // When calculating speeds, round up. This follows how we calculated
        // things in buttplug-js and buttplug-csharp, so it's more for history
        // than anything, but it's what users will expect.
        (speed_modifier + *self.rotation_step_ranges[index].start() as f64).ceil() as u32
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

    // If we're in a match all situation, set up the array with all prior
    // values before switching them out.
    if match_all && !result.iter().all(|x| x.is_none()) {
      for (index, rotation) in self.rotations.iter().enumerate() {
        if result[index].is_none() {
          result[index] = Some((rotation.0.load(SeqCst), rotation.1.load(SeqCst)));
        }
      }
    }

    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn _update_linear(&self, _msg: &LinearCmd) -> Result<Option<Vec<(u32, u32)>>, ButtplugError> {
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
    core::message::{ActuatorType, RotateCmd, RotationSubcommand, ScalarCmd, ScalarSubcommand},
    server::device::configuration::{
      ProtocolAttributesType,
      ServerDeviceMessageAttributesBuilder,
      ServerGenericDeviceMessageAttributes,
    },
  };
  use std::ops::RangeInclusive;

  #[test]
  pub fn test_command_generator_vibration() {
    let scalar_attrs = ServerGenericDeviceMessageAttributes::new(
      "Test",
      &RangeInclusive::new(0, 20),
      ActuatorType::Vibrate,
    );
    let scalar_attributes = ServerDeviceMessageAttributesBuilder::default()
      .scalar_cmd(&vec![scalar_attrs.clone(), scalar_attrs])
      .finish();
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      scalar_attributes,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);
    let vibrate_msg = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.5, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![
        Some((ActuatorType::Vibrate, 10)),
        Some((ActuatorType::Vibrate, 10))
      ]
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![]
    );
    let vibrate_msg_2 = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.75, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg_2, false)
        .expect("Test, assuming infallible"),
      vec![None, Some((ActuatorType::Vibrate, 15))]
    );
    let vibrate_msg_invalid = ScalarCmd::new(
      0,
      vec![ScalarSubcommand::new(2, 0.5, ActuatorType::Vibrate)],
    );
    assert!(mgr.update_scalar(&vibrate_msg_invalid, false).is_err());

    assert_eq!(
      mgr.scalars(),
      vec![
        Some((ActuatorType::Vibrate, 10)),
        Some((ActuatorType::Vibrate, 15))
      ]
    );
  }

  #[test]
  pub fn test_command_generator_vibration_match_all() {
    let scalar_attrs = ServerGenericDeviceMessageAttributes::new(
      "Test",
      &RangeInclusive::new(0, 20),
      ActuatorType::Vibrate,
    );
    let scalar_attributes = ServerDeviceMessageAttributesBuilder::default()
      .scalar_cmd(&vec![scalar_attrs.clone(), scalar_attrs])
      .finish();
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      scalar_attributes,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);
    let vibrate_msg = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.5, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, true)
        .expect("Test, assuming infallible"),
      vec![
        Some((ActuatorType::Vibrate, 10)),
        Some((ActuatorType::Vibrate, 10))
      ]
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, true)
        .expect("Test, assuming infallible"),
      vec![]
    );
    let vibrate_msg_2 = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.75, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg_2, true)
        .expect("Test, assuming infallible"),
      vec![
        Some((ActuatorType::Vibrate, 10)),
        Some((ActuatorType::Vibrate, 15))
      ]
    );
    let vibrate_msg_invalid = ScalarCmd::new(
      0,
      vec![ScalarSubcommand::new(2, 0.5, ActuatorType::Vibrate)],
    );
    assert!(mgr.update_scalar(&vibrate_msg_invalid, false).is_err());

    assert_eq!(
      mgr.scalars(),
      vec![
        Some((ActuatorType::Vibrate, 10)),
        Some((ActuatorType::Vibrate, 15))
      ]
    );
  }

  #[test]
  pub fn test_command_generator_vibration_step_range() {
    let mut vibrate_attrs_1 = ServerGenericDeviceMessageAttributes::new(
      "Test",
      &RangeInclusive::new(0, 20),
      ActuatorType::Vibrate,
    );
    vibrate_attrs_1.set_step_range(RangeInclusive::new(10, 15));
    let mut vibrate_attrs_2 = ServerGenericDeviceMessageAttributes::new(
      "Test",
      &RangeInclusive::new(0, 20),
      ActuatorType::Vibrate,
    );
    vibrate_attrs_2.set_step_range(RangeInclusive::new(10, 20));

    let vibrate_attributes = ServerDeviceMessageAttributesBuilder::default()
      .scalar_cmd(&vec![vibrate_attrs_1, vibrate_attrs_2])
      .finish();
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      vibrate_attributes,
      None,
    );
    let mgr = GenericCommandManager::new(&device_attributes);
    let vibrate_msg = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.5, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![
        Some((ActuatorType::Vibrate, 13)),
        Some((ActuatorType::Vibrate, 15))
      ]
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg, false)
        .expect("Test, assuming infallible"),
      vec![]
    );
    let vibrate_msg_2 = ScalarCmd::new(
      0,
      vec![
        ScalarSubcommand::new(0, 0.5, ActuatorType::Vibrate),
        ScalarSubcommand::new(1, 0.75, ActuatorType::Vibrate),
      ],
    );
    assert_eq!(
      mgr
        .update_scalar(&vibrate_msg_2, false)
        .expect("Test, assuming infallible"),
      vec![None, Some((ActuatorType::Vibrate, 18))]
    );
    let vibrate_msg_invalid = ScalarCmd::new(
      0,
      vec![ScalarSubcommand::new(2, 0.5, ActuatorType::Vibrate)],
    );
    assert!(mgr.update_scalar(&vibrate_msg_invalid, false).is_err());

    assert_eq!(
      mgr.scalars(),
      vec![
        Some((ActuatorType::Vibrate, 13)),
        Some((ActuatorType::Vibrate, 18))
      ]
    );
  }

  #[test]
  pub fn test_command_generator_rotation() {
    let rotate_attrs = ServerGenericDeviceMessageAttributes::new(
      "Test",
      &RangeInclusive::new(0, 20),
      ActuatorType::Rotate,
    );

    let rotate_attributes = ServerDeviceMessageAttributesBuilder::default()
      .rotate_cmd(&vec![rotate_attrs.clone(), rotate_attrs])
      .finish();
    let device_attributes = ProtocolDeviceAttributes::new(
      ProtocolAttributesType::Default,
      None,
      None,
      rotate_attributes,
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
        .update_rotation(&rotate_msg, false)
        .expect("Test, assuming infallible"),
      vec![Some((10, true)), Some((10, true))]
    );
    assert_eq!(
      mgr
        .update_rotation(&rotate_msg, false)
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
        .update_rotation(&rotate_msg_2, false)
        .expect("Test, assuming infallible"),
      vec![None, Some((15, false))]
    );
    let rotate_msg_3 = RotateCmd::new(
      0,
      vec![
        RotationSubcommand::new(0, 0.75, false),
        RotationSubcommand::new(1, 0.75, false),
      ],
    );
    assert_eq!(
      mgr
        .update_rotation(&rotate_msg_3, true)
        .expect("Test, assuming infallible"),
      vec![Some((15, false)), Some((15, false))]
    );
    let rotate_msg_invalid = RotateCmd::new(0, vec![RotationSubcommand::new(2, 0.5, true)]);
    assert!(mgr.update_rotation(&rotate_msg_invalid, false).is_err());
  }
  // TODO Write test for vibration stop generator
}
