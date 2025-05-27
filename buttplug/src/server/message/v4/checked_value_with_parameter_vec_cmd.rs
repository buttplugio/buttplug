use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator
    },
  },
  server::message::{v1::LinearCmdV1, ButtplugDeviceMessageTypeV3, RotateCmdV1, ServerDeviceAttributes, TryFromDeviceAttributes},
};
use getset::{Getters, CopyGetters};
use super::checked_value_with_parameter_cmd::CheckedValueWithParameterCmdV4;

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters, CopyGetters)] 
pub struct CheckedValueWithParameterVecCmdV4 {
  #[getset(get_copy="pub")]
  id: u32,
  #[getset(get_copy="pub")]
  device_index: u32,
  #[getset(get="pub")]
  value_vec: Vec<CheckedValueWithParameterCmdV4>
}

impl CheckedValueWithParameterVecCmdV4 {
  pub fn new(id: u32, device_index: u32, mut value_vec: Vec<CheckedValueWithParameterCmdV4>) -> Self {
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

impl ButtplugMessageValidator for CheckedValueWithParameterVecCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<LinearCmdV1> for CheckedValueWithParameterVecCmdV4 {
  fn try_from_device_attributes(
    msg: LinearCmdV1,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {

    let features = features
      .attrs_v3()
      .linear_cmd()
      .as_ref()
      .ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureMismatch("Device has no PositionWithDuration features".to_owned())))?;

    let mut cmds = vec!();
    for x in msg.vectors() {
      let f = features
          .get(x.index() as usize)
          .ok_or(ButtplugDeviceError::DeviceFeatureIndexError(features.len() as u32, x.index() as u32))?
          .feature();
      let actuator = f.actuator().as_ref().ok_or(ButtplugError::from(ButtplugDeviceError::DeviceFeatureMismatch("Device got LinearCmd command but has no actuators on Linear feature.".to_owned())))?;
      cmds.push(CheckedValueWithParameterCmdV4::new(
        msg.device_index(),
        x.index(),
        f.id(),
        crate::core::message::ActuatorType::PositionWithDuration,
        (x.position() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil() as u32,
        x.duration().try_into().map_err(|_| ButtplugError::from(ButtplugMessageError::InvalidMessageContents("Duration should be under 2^31. You are not waiting 24 days to run this command.".to_owned())))?,
      ));
    }
    Ok(CheckedValueWithParameterVecCmdV4::new(msg.id(), msg.device_index(), cmds))
  }
}

impl TryFromDeviceAttributes<RotateCmdV1> for CheckedValueWithParameterVecCmdV4 {
  // RotateCmd exists up through Message Spec v3. We can assume that, if we're receiving it, we can
  // just use the V3 spec client device attributes for it. If this was sent on a V1/V2 protocol,
  // it'll still have all the same features.
  fn try_from_device_attributes(
    msg: RotateCmdV1,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedValueWithParameterCmdV4> = vec![];
    for cmd in msg.rotations() {
      let rotate_attrs = attrs
        .attrs_v3()
        .rotate_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageTypeV3::RotateCmd.to_string(),
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
        .actuator()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("RotateCmdV1".to_owned()),
        ))?;
      cmds.push(CheckedValueWithParameterCmdV4::new(
        msg.device_index(),
        idx,
        feature.feature.id(),
        crate::core::message::ActuatorType::RotateWithDirection,
        (cmd.speed() * ((*actuator.step_limit().end() - *actuator.step_limit().start()) as f64) + *actuator.step_limit().start() as f64).ceil() as u32,
        if cmd.clockwise() { 1 } else { -1 }
      ));
    }
    Ok(CheckedValueWithParameterVecCmdV4::new(msg.id(), msg.device_index(), cmds))
  }
}
