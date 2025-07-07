// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v0::SingleMotorVibrateCmdV0,
  v1::VibrateCmdV1,
  v3::ScalarCmdV3,
  ButtplugDeviceMessageNameV3,
  LinearCmdV1,
  RotateCmdV1,
  ServerDeviceAttributes,
  TryFromDeviceAttributes,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    OutputCommand,
    OutputPositionWithDuration,
    OutputRotateWithDirection,
    OutputType,
    OutputValue,
  },
};
use getset::{CopyGetters, Getters};

use super::checked_output_cmd::CheckedOutputCmdV4;

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
pub struct CheckedOutputVecCmdV4 {
  #[getset(get_copy = "pub")]
  id: u32,
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[getset(get = "pub")]
  value_vec: Vec<CheckedOutputCmdV4>,
}

impl CheckedOutputVecCmdV4 {
  pub fn new(id: u32, device_index: u32, mut value_vec: Vec<CheckedOutputCmdV4>) -> Self {
    // Several tests and parts of the system assumed we always sorted by feature index. This is not
    // necessarily true of incoming messages, but we also never explicitly specified the execution
    // order of subcommands within a message, so we'll just sort here for now to make tests pass,
    // and implement unordered checking after v4 ships.
    value_vec.sort_by_key(|k| k.feature_index());
    Self {
      id,
      device_index,
      value_vec,
    }
  }
}

impl ButtplugMessageValidator for CheckedOutputVecCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<SingleMotorVibrateCmdV0> for CheckedOutputVecCmdV4 {
  // For VibrateCmd, just take everything out of V2's VibrateCmd and make a command.
  fn try_from_device_attributes(
    msg: SingleMotorVibrateCmdV0,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    let mut vibrate_features = features
      .features()
      .iter()
      .enumerate()
      .filter(|(_, feature)| {
        if let Some(output_map) = feature.output() {
          output_map.contains_key(&buttplug_core::message::OutputType::Vibrate)
        } else {
          false
        }
      })
      .peekable();

    // Check to make sure we have any vibrate attributes at all.
    if vibrate_features.peek().is_none() {
      return Err(
        ButtplugDeviceError::DeviceFeatureMismatch("Device has no Vibrate features".to_owned())
          .into(),
      );
    }

    let mut cmds = vec![];
    for (index, feature) in vibrate_features {
      // if we've made it this far, we know we have actuators in a list
      let actuator = feature
        .output()
        .as_ref()
        .unwrap()
        .get(&OutputType::Vibrate)
        .unwrap();
      // This doesn't need to run through a security check because we have to construct it to be
      // inherently secure anyways.
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        index as u32,
        feature.id(),
        OutputCommand::Vibrate(OutputValue::new(
          (msg.speed() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64)
            + *actuator.step_limit().start() as f64)
            .ceil() as u32,
        )),
      ))
    }
    Ok(CheckedOutputVecCmdV4::new(
      msg.id(),
      msg.device_index(),
      cmds,
    ))
  }
}

impl TryFromDeviceAttributes<VibrateCmdV1> for CheckedOutputVecCmdV4 {
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
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    let vibrate_attributes =
      features
        .attrs_v2()
        .vibrate_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureCountMismatch(0, msg.speeds().len() as u32),
        ))?;

    let mut cmds: Vec<CheckedOutputCmdV4> = vec![];
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
      let actuator = feature
        .output()
        .as_ref()
        .ok_or(ButtplugDeviceError::DeviceConfigurationError(
          "Device configuration does not have Vibrate actuator available.".to_owned(),
        ))?
        .get(&OutputType::Vibrate)
        .ok_or(ButtplugDeviceError::DeviceConfigurationError(
          "Device configuration does not have Vibrate actuator available.".to_owned(),
        ))?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx as u32,
        feature.id(),
        OutputCommand::Vibrate(OutputValue::new(
          (vibrate_cmd.speed()
            * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64)
            + *actuator.step_limit().start() as f64)
            .ceil() as u32,
        )),
      ))
    }
    Ok(CheckedOutputVecCmdV4::new(
      msg.id(),
      msg.device_index(),
      cmds,
    ))
  }
}

impl TryFromDeviceAttributes<ScalarCmdV3> for CheckedOutputVecCmdV4 {
  // ScalarCmd only came in with V3, so we can just use the V3 device attributes.
  fn try_from_device_attributes(
    msg: ScalarCmdV3,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedOutputCmdV4> = vec![];
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
        .output()
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
        cmds.push(CheckedOutputCmdV4::new(
          msg.id(),
          msg.device_index(),
          idx,
          feature.feature.id(),
          OutputCommand::from_output_type(
            cmd.actuator_type(),
            (cmd.scalar()
              * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64)
              + *actuator.step_limit().start() as f64)
              .ceil() as u32,
          )
          .unwrap(),
        ));
      } else {
        cmds.push(CheckedOutputCmdV4::new(
          msg.id(),
          msg.device_index(),
          idx,
          feature.feature.id(),
          OutputCommand::from_output_type(cmd.actuator_type(), 0).unwrap(),
        ));
      }
    }

    Ok(CheckedOutputVecCmdV4::new(
      msg.id(),
      msg.device_index(),
      cmds,
    ))
  }
}

impl TryFromDeviceAttributes<LinearCmdV1> for CheckedOutputVecCmdV4 {
  fn try_from_device_attributes(
    msg: LinearCmdV1,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    let features = features
      .attrs_v3()
      .linear_cmd()
      .as_ref()
      .ok_or(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureMismatch(
          "Device has no PositionWithDuration features".to_owned(),
        ),
      ))?;

    let mut cmds = vec![];
    for x in msg.vectors() {
      let f = features
        .get(x.index() as usize)
        .ok_or(ButtplugDeviceError::DeviceFeatureIndexError(
          features.len() as u32,
          x.index(),
        ))?
        .feature();
      let actuator = f
        .output()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureMismatch(
            "Device got LinearCmd command but has no actuators on Linear feature.".to_owned(),
          ),
        ))?
        .get(&buttplug_core::message::OutputType::PositionWithDuration)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureMismatch(
            "Device got LinearCmd command but has no actuators on Linear feature.".to_owned(),
          ),
        ))?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.device_index(),
        x.index(),
        0,
        f.id(),
        OutputCommand::PositionWithDuration(OutputPositionWithDuration::new(
          (x.position() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64)
            + *actuator.step_limit().start() as f64)
            .ceil() as u32,
          x.duration().try_into().map_err(|_| {
            ButtplugError::from(ButtplugMessageError::InvalidMessageContents(
              "Duration should be under 2^31. You are not waiting 24 days to run this command."
                .to_owned(),
            ))
          })?,
        )),
      ));
    }
    Ok(CheckedOutputVecCmdV4::new(
      msg.id(),
      msg.device_index(),
      cmds,
    ))
  }
}

impl TryFromDeviceAttributes<RotateCmdV1> for CheckedOutputVecCmdV4 {
  // RotateCmd exists up through Message Spec v3. We can assume that, if we're receiving it, we can
  // just use the V3 spec client device attributes for it. If this was sent on a V1/V2 protocol,
  // it'll still have all the same features.
  fn try_from_device_attributes(
    msg: RotateCmdV1,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, buttplug_core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedOutputCmdV4> = vec![];
    for cmd in msg.rotations() {
      let rotate_attrs = attrs
        .attrs_v3()
        .rotate_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageNameV3::RotateCmd.to_string(),
          ),
        ))?;
      let feature = rotate_attrs
        .get(cmd.index() as usize)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureIndexError(rotate_attrs.len() as u32, cmd.index()),
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
        .output()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("RotateCmdV1".to_owned()),
        ))?
        .get(&buttplug_core::message::OutputType::RotateWithDirection)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("RotateCmdV1".to_owned()),
        ))?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx,
        feature.feature.id(),
        OutputCommand::RotateWithDirection(OutputRotateWithDirection::new(
          (cmd.speed() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64)
            + *actuator.step_limit().start() as f64)
            .ceil() as u32,
          cmd.clockwise(),
        )),
      ));
    }
    Ok(CheckedOutputVecCmdV4::new(
      msg.id(),
      msg.device_index(),
      cmds,
    ))
  }
}
