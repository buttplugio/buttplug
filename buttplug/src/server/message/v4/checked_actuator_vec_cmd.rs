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
      ActuatorType, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator
    },
  },
  server::message::{
    v0::SingleMotorVibrateCmdV0, v1::VibrateCmdV1, v3::ScalarCmdV3, ButtplugDeviceMessageNameV3, ServerDeviceAttributes, TryFromDeviceAttributes
  },
};
use getset::{CopyGetters, Getters};

use super::checked_actuator_cmd::CheckedActuatorCmdV4;

#[derive(
  Debug,
  Default,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Clone,
  Getters,
  CopyGetters,
)]
pub struct CheckedActuatorVecCmdV4 {
  #[getset(get_copy = "pub")]
  id: u32,
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[getset(get = "pub")]
  value_vec: Vec<CheckedActuatorCmdV4>
}

impl CheckedActuatorVecCmdV4 {
  pub fn new(id: u32, device_index: u32, mut value_vec: Vec<CheckedActuatorCmdV4>) -> Self {
    // Several tests and parts of the system assumed we always sorted by feature index. This is not
    // necessarily true of incoming messages, but we also never explicitly specified the execution
    // order of subcommands within a message, so we'll just sort here for now to make tests pass,
    // and implement unordered checking after v4 ships.
    value_vec.sort_by_key(|k| k.feature_index());
    Self {
      id,
      device_index,
      value_vec
    }
  }
}

impl ButtplugMessageValidator for CheckedActuatorVecCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}


impl TryFromDeviceAttributes<SingleMotorVibrateCmdV0> for CheckedActuatorVecCmdV4 {
  // For VibrateCmd, just take everything out of V2's VibrateCmd and make a command.
  fn try_from_device_attributes(
    msg: SingleMotorVibrateCmdV0,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut vibrate_features = features
      .features()
      .iter()
      .enumerate()
      .filter(|(_, feature)| {
        if let Some(actuator_map) = feature.actuator() {
          actuator_map.contains_key(&crate::core::message::ActuatorType::Vibrate)
        } else {
          false
        }
      })
      .peekable();

    // Check to make sure we have any vibrate attributes at all.
    if vibrate_features.peek().is_none() {
      return Err(ButtplugDeviceError::DeviceFeatureMismatch("Device has no Vibrate features".to_owned()).into());
    }

    let mut cmds = vec!();
    for (index, feature) in vibrate_features {
      // if we've made it this far, we know we have actuators in a list
      let actuator = feature.actuator().as_ref().unwrap().get(&ActuatorType::Vibrate).unwrap();
      // This doesn't need to run through a security check because we have to construct it to be
      // inherently secure anyways.
      cmds.push(CheckedActuatorCmdV4::new(
        msg.id(),
        msg.device_index(),
        index as u32,
        feature.id(),
        crate::core::message::ActuatorType::Vibrate,
        (msg.speed() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil() as u32,
      ))
    }
    Ok(CheckedActuatorVecCmdV4::new(msg.id(), msg.device_index(), cmds))
  }
}

impl TryFromDeviceAttributes<VibrateCmdV1> for CheckedActuatorVecCmdV4 {
  // VibrateCmd only exists up through Message Spec v2. We can assume that, if we're receiving it,
  // we can just use the V2 spec client device attributes for it. If this was sent on a V1 protocol,
  // it'll still have all the same features.
  //
  // Due to specs v1/2 using feature counts instead of per-feature objects, we calculate our indexes
  // based on the feature counts in our current device definitions, as that's how we generate them
  // on the way out.
  fn try_from_device_attributes(
    msg: VibrateCmdV1,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let vibrate_attributes =
      features
        .attrs_v2()
        .vibrate_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureCountMismatch(0, msg.speeds().len() as u32),
        ))?;

    let mut cmds: Vec<CheckedActuatorCmdV4> = vec![];
    for vibrate_cmd in msg.speeds() {
      if vibrate_cmd.index() > vibrate_attributes.features().len() as u32 {
        return Err(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureCountMismatch(
            vibrate_cmd.index(),
            msg.speeds().len() as u32,
          ),
        ));
      }
      let feature = &vibrate_attributes.features()[vibrate_cmd.index() as usize];
      let idx = features
        .features()
        .iter()
        .enumerate()
        .find(|(_, f)| f.id() == feature.id())
        .expect("Already checked existence")
        .0;
      let actuator =
        feature
          .actuator()
          .as_ref()
          .ok_or(ButtplugDeviceError::DeviceConfigurationError(
            "Device configuration does not have Vibrate actuator available.".to_owned(),
          ))?
          .get(&ActuatorType::Vibrate)
          .ok_or(ButtplugDeviceError::DeviceConfigurationError(
            "Device configuration does not have Vibrate actuator available.".to_owned(),
          ))?;
      cmds.push(CheckedActuatorCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx as u32,
        feature.id(),
        crate::core::message::ActuatorType::Vibrate,
        (vibrate_cmd.speed() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil() as u32,
      ))
    }
    Ok(CheckedActuatorVecCmdV4::new(msg.id(), msg.device_index(), cmds))
  }
}

impl TryFromDeviceAttributes<ScalarCmdV3> for CheckedActuatorVecCmdV4 {
  // ScalarCmd only came in with V3, so we can just use the V3 device attributes.
  fn try_from_device_attributes(
    msg: ScalarCmdV3,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedActuatorCmdV4> = vec![];
    if msg.scalars().is_empty() {
      return Err(ButtplugError::from(
        ButtplugDeviceError::ProtocolRequirementError(
          "ScalarCmd with no subcommands is not allowed.".to_owned(),
        ),
      ));
    }
    for cmd in msg.scalars() {
      let scalar_attrs = attrs
        .attrs_v3()
        .scalar_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageNameV3::ScalarCmd.to_string(),
          ),
        ))?;
      let feature = scalar_attrs
        .get(cmd.index() as usize)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureIndexError(scalar_attrs.len() as u32, cmd.index()),
        ))?;
      let idx = attrs
        .features()
        .iter()
        .enumerate()
        .find(|(_, f)| f.id() == feature.feature().id())
        .expect("Already proved existence")
        .0 as u32;
      let actuator = feature
        .feature()
        .actuator()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("ScalarCmdV3".to_owned()),
        ))?
        .get(&cmd.actuator_type())
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("ScalarCmdV3".to_owned()),
        ))?;

      // This needs to take the user configured step limit into account, otherwise we'll hand back
      // the wrong placement and it won't be noticed.
      if cmd.scalar() > 0.000001 {
        cmds.push(CheckedActuatorCmdV4::new(
          msg.id(),
          msg.device_index(),
          idx,
          feature.feature.id(),
          cmd.actuator_type(),
          (cmd.scalar() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil() as u32,
        ));
      } else {
        cmds.push(CheckedActuatorCmdV4::new(
          msg.id(),
          msg.device_index(),
          idx,
          feature.feature.id(),
          cmd.actuator_type(),
          0
        ));
      }
    }

    Ok(CheckedActuatorVecCmdV4::new(msg.id(), msg.device_index(), cmds))
  }
}
