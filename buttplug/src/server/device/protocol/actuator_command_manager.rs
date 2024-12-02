// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugError,
    message::{
      ActuatorType,
      ButtplugActuatorFeatureMessageType,
      DeviceFeatureActuator,
    },
  },
  server::message::{
    checked_level_cmd::{CheckedLevelCmdV4, CheckedLevelSubcommandV4}, server_device_feature::ServerDeviceFeature, spec_enums::ButtplugDeviceCommandMessageUnion
  },
};
use getset::Getters;
use std::collections::HashMap;
use std::{
  collections::HashSet,
  sync::atomic::{AtomicBool, AtomicI32, Ordering::Relaxed},
};
use uuid::Uuid;

#[derive(Getters)]
#[getset(get = "pub")]
struct FeatureStatus {
  feature_id: Uuid,
  actuator_type: ActuatorType,
  actuator: DeviceFeatureActuator,
  sent: AtomicBool,
  value: AtomicI32,
}

impl FeatureStatus {
  pub fn new(
    feature_id: &Uuid,
    actuator_type: &ActuatorType,
    actuator: &DeviceFeatureActuator,
  ) -> Self {
    Self {
      feature_id: *feature_id,
      actuator_type: *actuator_type,
      actuator: actuator.clone(),
      sent: AtomicBool::new(false),
      value: AtomicI32::new(0),
    }
  }

  pub fn current(&self) -> (ActuatorType, i32) {
    (self.actuator_type, self.value.load(Relaxed))
  }

  pub fn messages(&self) -> &HashSet<ButtplugActuatorFeatureMessageType> {
    self.actuator.messages()
  }

  pub fn update(&self, value: i32) -> Option<i32> {
    let mut result = None;
    let range_start = *self.actuator.step_limit().start();
    let range = self.actuator.step_limit().end() - range_start;

    trace!(
      "{:?} {:?} {}",
      self.actuator.step_range(),
      self.actuator.step_limit(),
      range,
    );
    // If we've already sent commands, we don't want to send them again,
    // because some of our communication busses are REALLY slow. Make sure
    // these values get None in our return vector.
    let current = self.value.load(Relaxed);
    let sent = self.sent.load(Relaxed);
    if !sent || value != current {
      self.value.store(value, Relaxed);
      if !sent {
        self.sent.store(true, Relaxed);
      }
      result = Some(value);
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
  pub fn new(features: &Vec<ServerDeviceFeature>) -> Self {
    let mut stop_commands = vec![];

    let mut statuses = vec![];
    let mut level_subcommands = vec![];
    for (index, feature) in features.iter().enumerate() {
      if let Some(actuator) = feature.actuator() {
        let actuator_type: ActuatorType = (*feature.feature_type()).try_into().unwrap();
        statuses.push(FeatureStatus::new(feature.id(), &actuator_type, actuator));
        if actuator
          .messages()
          .contains(&crate::core::message::ButtplugActuatorFeatureMessageType::LevelCmd)
        {
          level_subcommands.push(CheckedLevelSubcommandV4::new(
            index as u32,
            0,
            *feature.id(),
          ));
        }
      }
    }
    if !level_subcommands.is_empty() {
      stop_commands.push(CheckedLevelCmdV4::new(0, 0, &level_subcommands).into());
    }

    Self {
      feature_status: statuses,
      stop_commands,
    }
  }

  fn update(
    &self,
    msg_type: ButtplugActuatorFeatureMessageType,
    commands: &Vec<(Uuid, ActuatorType, i32)>,
    match_all: bool,
  ) -> Result<Vec<(Uuid, ActuatorType, i32)>, ButtplugError> {
    // Convert from the generic 0.0-1.0 range to the StepCount attribute given by the device config.

    // If we've already sent commands before, we should check against our old values. Otherwise, we
    // should always send whatever command we're going to send.
    let mut result = vec![];

    for cmd in self.feature_status.iter() {
      if let Some((_, actuator, cmd_value)) = commands.iter().find(|x| x.0 == *cmd.feature_id()) {
        // By this point, we should have already checked whether the feature takes the message type.
        if let Some(updated_value) = cmd.update(*cmd_value) {
          result.push((cmd.feature_id().clone(), *actuator, updated_value));
        } else if match_all {
          result.push((cmd.feature_id().clone(), *actuator, cmd.current().1));
        }
      } else if match_all && cmd.messages().contains(&msg_type) {
        result.push((
          cmd.feature_id().clone(),
          *cmd.actuator_type(),
          cmd.current().1,
        ));
      }
    }
    // Return the command vector for the protocol to turn into proprietary commands
    Ok(result)
  }

  pub fn update_level(
    &self,
    msg: &CheckedLevelCmdV4,
    match_all: bool,
  ) -> Result<Vec<Option<(ActuatorType, i32)>>, ButtplugError> {
    trace!("Updating level for message: {:?}", msg);

    let mut idxs = HashMap::new();
    for x in self.feature_status.iter() {
      if x
        .messages()
        .contains(&ButtplugActuatorFeatureMessageType::LevelCmd)
      {
        idxs.insert(x.feature_id(), idxs.len() as u32);
      }
    }

    let mut final_result = vec![None; idxs.len()];

    let mut commands = vec![];
    msg.levels().iter().for_each(|x| {
      let id = x.feature_id();
      trace!("Updating command for {:?}", id);
      commands.push((
        id.clone(),
        *self
          .feature_status
          .iter()
          .find(|y| *y.feature_id() == x.feature_id())
          .unwrap()
          .actuator_type(),
        x.level(),
      ))
    });
    let mut result = self.update(
      ButtplugActuatorFeatureMessageType::LevelCmd,
      &commands,
      match_all,
    )?;
    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    result.iter().for_each(|(index, actuator, value)| {
      final_result[*idxs.get(index).unwrap() as usize] = Some((*actuator, *value))
    });
    debug!("{:?}", final_result);
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
