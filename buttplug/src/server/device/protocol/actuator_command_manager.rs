// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    ActuatorType,
    ButtplugActuatorFeatureMessageType,
    ButtplugDeviceCommandMessageUnion,
    DeviceFeature,
    DeviceFeatureActuator,
    RotateCmdV2,
    RotationSubcommandV2,
    ScalarCmdV3,
    ScalarCmdV4,
    ScalarSubcommandV3,
  },
};
use getset::Getters;
use std::{
  collections::HashSet,
  sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed},
};

// As of the last rewrite of the command manager, we're currently only tracking values of scalar and
// rotation commands. We can just use the rotation (AtomicU32, AtomicBool) pair for storage, and
// ignore the direction bool for Scalars.
#[derive(Getters)]
#[getset(get = "pub")]
struct FeatureStatus {
  actuator_type: ActuatorType,
  actuator: DeviceFeatureActuator,
  sent: AtomicBool,
  value: (AtomicU32, AtomicBool),
}

impl FeatureStatus {
  pub fn new(actuator_type: &ActuatorType, actuator: &DeviceFeatureActuator) -> Self {
    Self {
      actuator_type: *actuator_type,
      actuator: actuator.clone(),
      sent: AtomicBool::new(false),
      value: (AtomicU32::new(0), AtomicBool::new(false)),
    }
  }

  pub fn current(&self) -> (ActuatorType, (u32, bool)) {
    (
      self.actuator_type,
      (self.value.0.load(Relaxed), self.value.1.load(Relaxed)),
    )
  }

  pub fn messages(&self) -> &HashSet<ButtplugActuatorFeatureMessageType> {
    self.actuator.messages()
  }

  pub fn update(&self, value: &(f64, bool)) -> Option<(u32, bool)> {
    let mut result = None;
    let range_start = *self.actuator.step_range().start();
    let range = self.actuator.step_range().end() - range_start;
    let scalar_modifier = value.0 * range as f64;
    let scalar = if scalar_modifier < 0.0001 {
      0
    } else {
      // When calculating speeds, round up. This follows how we calculated
      // things in buttplug-js and buttplug-csharp, so it's more for history
      // than anything, but it's what users will expect.
      (scalar_modifier + range_start as f64).ceil() as u32
    };
    trace!(
      "{:?} {} {} {}",
      self.actuator.step_range(),
      range,
      scalar_modifier,
      scalar
    );
    // If we've already sent commands, we don't want to send them again,
    // because some of our communication busses are REALLY slow. Make sure
    // these values get None in our return vector.
    let current = self.value.0.load(Relaxed);
    let clockwise = self.value.1.load(Relaxed);
    let sent = self.sent.load(Relaxed);
    if !sent || scalar != current || clockwise != value.1 {
      self.value.0.store(scalar, Relaxed);
      self.value.1.store(value.1, Relaxed);
      if !sent {
        self.sent.store(true, Relaxed);
      }
      result = Some((scalar, value.1));
    }
    result
  }
}

// In order to make our lives easier, we make some assumptions about what's internally mutable in
// the ActuatorCommandManager (ACM). Once the ACM is configured for a device, it won't change sizes,
// because we don't support things like adding motors to devices randomly while Buttplug is running.
// Therefore we know that we'll just be storing values like vibration/rotation speeds. We can assume
// our storage of those can stay immutable (the vec sizes won't change) and make their internals
// mutable. While this could be RefCell'd or whatever, they're also always atomic types (until the
// horrible day some sex toy decides to use floats in its protocol), so we can just use atomics and
// call it done.
pub struct ActuatorCommandManager {
  feature_status: Vec<FeatureStatus>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ActuatorCommandManager {
  pub fn new(features: &Vec<DeviceFeature>) -> Self {
    let mut stop_commands = vec![];

    let mut statuses = vec![];
    let mut scalar_subcommands = vec![];
    let mut rotate_subcommands = vec![];
    for (index, feature) in features.iter().enumerate() {
      if let Some(actuator) = feature.actuator() {
        let actuator_type: ActuatorType = feature.feature_type().clone().try_into().unwrap();
        statuses.push(FeatureStatus::new(&actuator_type, actuator));
        if actuator
          .messages()
          .contains(&crate::core::message::ButtplugActuatorFeatureMessageType::RotateCmd)
        {
        rotate_subcommands.push(RotationSubcommandV2::new(index as u32, 0.0, false));
        } else if actuator
          .messages()
          .contains(&crate::core::message::ButtplugActuatorFeatureMessageType::ScalarCmd)
        {
          scalar_subcommands.push(ScalarSubcommandV3::new(index as u32, 0.0, actuator_type));
        }
      }
    }
    if !scalar_subcommands.is_empty() {
      stop_commands.push(ScalarCmdV3::new(0, scalar_subcommands).into());
    }
    if !rotate_subcommands.is_empty() {
      stop_commands.push(RotateCmdV2::new(0, rotate_subcommands).into());
    }

    error!("{:?}", stop_commands);

    Self {
      feature_status: statuses,
      stop_commands,
    }
  }

  fn update(
    &self,
    msg_type: ButtplugActuatorFeatureMessageType,
    commands: &Vec<(u32, ActuatorType, (f64, bool))>,
    match_all: bool,
  ) -> Result<Vec<(u32, ActuatorType, (u32, bool))>, ButtplugError> {
    // Convert from the generic 0.0-1.0 range to the StepCount attribute given by the device config.

    // If we've already sent commands before, we should check against our old values. Otherwise, we
    // should always send whatever command we're going to send.
    let mut result: Vec<(u32, ActuatorType, (u32, bool))> = vec![];

    for command in commands {
      if command.0 >= self.feature_status.len().try_into().unwrap() {
        return Err(
          ButtplugDeviceError::ProtocolRequirementError(format!(
            "Command requests feature index {}, which does not exist.",
            command.0,
          ))
          .into(),
        );
      }
    }

    for (index, cmd) in self.feature_status.iter().enumerate() {
      let u32_index: u32 = index.try_into().unwrap();
      if let Some((_, cmd_actuator, cmd_value)) = commands.iter().find(|x| x.0 == u32_index) {
        // By this point, we should have already checked whether the feature takes the message type.
        if let Some(updated_value) = self.feature_status[index].update(cmd_value) {
          result.push((u32_index, *cmd_actuator, updated_value));
        } else if match_all {
          result.push((u32_index, *cmd.actuator_type(), cmd.current().1));
        }
      } else if match_all {
        if cmd.messages().contains(&msg_type) {
          result.push((u32_index, *cmd.actuator_type(), cmd.current().1));
        }
      }
    }
    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn update_scalar(
    &self,
    msg: &ScalarCmdV4,
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

    let mut final_result: Vec<Option<(ActuatorType, u32)>> = vec![
      None;
      self
        .feature_status
        .iter()
        .filter(|x| x
          .messages()
          .contains(&ButtplugActuatorFeatureMessageType::ScalarCmd))
        .count()
    ];

    let mut commands: Vec<(u32, ActuatorType, (f64, bool))> = vec![];
    msg
      .scalars()
      .iter()
      .for_each(|x| commands.push((x.feature_index(), x.actuator_type(), (x.scalar(), false))));
    let mut result = self.update(
      ButtplugActuatorFeatureMessageType::ScalarCmd,
      &commands,
      match_all,
    )?;
    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    result.iter().for_each(|(index, actuator, value)| {
      final_result[*index as usize] = Some((*actuator, value.0))
    });
    error!("{:?}", final_result);
    Ok(final_result)
  }

  pub fn update_rotation(
    &self,
    msg: &RotateCmdV2,
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

    let mut final_result: Vec<Option<(u32, bool)>> = vec![
      None;
      self
        .feature_status
        .iter()
        .filter(|x| x
          .messages()
          .contains(&ButtplugActuatorFeatureMessageType::RotateCmd))
        .count()
    ];

    let mut commands: Vec<(u32, ActuatorType, (f64, bool))> = vec![];
    msg
      .rotations()
      .iter()
      .for_each(|x| commands.push((x.index(), ActuatorType::Rotate, (x.speed(), x.clockwise()))));
    let mut result = self.update(
      ButtplugActuatorFeatureMessageType::RotateCmd,
      &commands,
      match_all,
    )?;
    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    result
      .iter()
      .enumerate()
      .for_each(|(array_index, (_, _, value))| final_result[array_index] = Some(*value));
    Ok(final_result)
  }

  pub fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
    self.stop_commands.clone()
  }
}
/*
#[cfg(test)]
mod test {
  use super::{GenericCommandManager, ProtocolDeviceAttributes};
  use crate::{
    core::message::{ActuatorType, RotateCmd, RotationSubcommand, ScalarCmd, ScalarSubcommand},
    server::device::configuration::{
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
    let device_attributes = ProtocolDeviceAttributes::new("Whatever", &None, &scalar_attributes);
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
    let device_attributes = ProtocolDeviceAttributes::new("Whatever", &None, &scalar_attributes);
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
    let device_attributes = ProtocolDeviceAttributes::new("Whatever", &None, &vibrate_attributes);
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
    let device_attributes = ProtocolDeviceAttributes::new("Whatever", &None, &rotate_attributes);
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
*/
