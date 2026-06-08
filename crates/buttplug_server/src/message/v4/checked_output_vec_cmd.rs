// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugDeviceMessageNameV3,
  LinearCmdV1,
  RotateCmdV1,
  ServerDeviceAttributes,
  TryFromDeviceAttributes,
  v0::SingleMotorVibrateCmdV0,
  v1::VibrateCmdV1,
  v3::ScalarCmdV3,
};
use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageValidator,
    OutputCommand,
    OutputHwPositionWithDuration,
    OutputType,
    OutputValue,
  },
};
use buttplug_server_device_config::ServerDeviceFeatureOutput;
use getset::{CopyGetters, Getters};

use super::checked_output_cmd::CheckedOutputCmdV4;

fn feature_index_for_id(
  attrs: &ServerDeviceAttributes,
  feature_id: uuid::Uuid,
  command_name: &str,
) -> Result<u32, ButtplugError> {
  attrs
    .features()
    .iter()
    .find(|(_, f)| f.id() == feature_id)
    .map(|(idx, _)| *idx)
    .ok_or_else(|| {
      ButtplugDeviceError::DeviceConfigurationError(format!(
        "Feature {feature_id} referenced by {command_name} was not found in device attributes."
      ))
      .into()
    })
}

#[derive(Debug, Default, PartialEq, Clone, Getters, CopyGetters)]
pub struct CheckedOutputVecCmdV4 {
  #[getset(get_copy = "pub")]
  id: u32,
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[getset(get = "pub")]
  value_vec: Vec<CheckedOutputCmdV4>,
}

impl ButtplugMessage for CheckedOutputVecCmdV4 {
  fn id(&self) -> u32 {
    self.id
  }
  fn set_id(&mut self, id: u32) {
    self.id = id;
  }
}

impl ButtplugDeviceMessage for CheckedOutputVecCmdV4 {
  fn device_index(&self) -> u32 {
    self.device_index
  }
  fn set_device_index(&mut self, device_index: u32) {
    self.device_index = device_index;
  }
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
      .filter(|(_, feature)| feature.contains_output(OutputType::Vibrate))
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
        .get_output(OutputType::Vibrate)
        .expect("Already confirmed we have vibrator for this feature");
      // This doesn't need to run through a security check because we have to construct it to be
      // inherently secure anyways.
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        *index,
        feature.id(),
        OutputCommand::Vibrate(OutputValue::new(
          actuator.calculate_from_float(msg.speed()).map_err(
            |e: buttplug_server_device_config::ButtplugDeviceConfigError| {
              ButtplugMessageError::InvalidMessageContents(e.to_string())
            },
          )?,
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
      let feature = vibrate_attributes
        .features()
        .get(vibrate_cmd.index() as usize)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureIndexError(
            vibrate_attributes.features().len() as u32,
            vibrate_cmd.index(),
          ),
        ))?;
      let idx = feature_index_for_id(features, feature.id(), "VibrateCmdV1")?;
      let actuator = feature.get_output(OutputType::Vibrate).ok_or(
        ButtplugDeviceError::DeviceConfigurationError(
          "Device configuration does not have Vibrate actuator available.".to_owned(),
        ),
      )?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx,
        feature.id(),
        OutputCommand::Vibrate(OutputValue::new(
          actuator
            .calculate_from_float(vibrate_cmd.speed())
            .map_err(|e| ButtplugMessageError::InvalidMessageContents(e.to_string()))?,
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
      let scalar_attrs = if let Some(a) = attrs.attrs_v3().scalar_cmd() {
        a
      } else {
        continue;
      };
      let feature = scalar_attrs
        .get(cmd.index() as usize)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureIndexError(scalar_attrs.len() as u32, cmd.index()),
        ))?;
      let idx = feature_index_for_id(attrs, feature.feature().id(), "ScalarCmdV3")?;
      let output = feature
        .feature()
        .get_output(cmd.actuator_type())
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported("ScalarCmdV3".to_owned()),
        ))?;
      let output_value = output.calculate_from_float(cmd.scalar()).map_err(|e| {
        error!("{:?}", e);
        ButtplugError::from(ButtplugDeviceError::MessageNotSupported(
          "ScalarCmdV3".to_owned(),
        ))
      })?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx,
        feature.feature.id(),
        OutputCommand::from_output_type(cmd.actuator_type(), output_value).unwrap(),
      ));
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
      let hw_pos = f
        .get_output(OutputType::HwPositionWithDuration)
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceFeatureMismatch(
            "Device got LinearCmd command but has no actuators on Linear feature.".to_owned(),
          ),
        ))?;
      let actuator = if let ServerDeviceFeatureOutput::HwPositionWithDuration(p) = hw_pos {
        p
      } else {
        unreachable!("get_output(HwPositionWithDuration) always returns HwPositionWithDuration")
      };
      cmds.push(CheckedOutputCmdV4::new(
        msg.device_index(),
        x.index(),
        0,
        f.id(),
        OutputCommand::HwPositionWithDuration(OutputHwPositionWithDuration::new(
          actuator.calculate_scaled_float(x.position()).map_err(|_| {
            ButtplugError::from(ButtplugMessageError::InvalidMessageContents(
              "Position should be 0.0 < x < 1.0".to_owned(),
            ))
          })?,
          x.duration(),
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
      let idx = feature_index_for_id(attrs, feature.feature().id(), "RotateCmdV1")?;
      let actuator =
        feature
          .feature()
          .get_output(OutputType::Rotate)
          .ok_or(ButtplugError::from(
            ButtplugDeviceError::MessageNotSupported("RotateCmdV1".to_owned()),
          ))?;
      cmds.push(CheckedOutputCmdV4::new(
        msg.id(),
        msg.device_index(),
        idx,
        feature.feature.id(),
        OutputCommand::Rotate(OutputValue::new(
          actuator.calculate_from_float(cmd.speed()).map_err(|_| {
            ButtplugError::from(ButtplugMessageError::InvalidMessageContents(
              "Position should be 0.0 < x < 1.0".to_owned(),
            ))
          })?
            * (if cmd.clockwise() { 1 } else { -1 }),
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::message::v1::VibrateSubcommandV1;
  use buttplug_core::util::{range::RangeInclusive, small_vec_enum_map::SmallVecEnumMap};
  use buttplug_server_device_config::{
    RangeWithLimit,
    ServerDeviceFeature,
    ServerDeviceFeatureOutputValueProperties,
  };
  use std::collections::BTreeMap;
  use uuid::Uuid;

  fn attrs_with_one_vibrate_feature() -> ServerDeviceAttributes {
    let output = vec![ServerDeviceFeatureOutput::Vibrate(
      ServerDeviceFeatureOutputValueProperties::new(
        RangeWithLimit::new(RangeInclusive::new(0, 100)),
        false,
      ),
    )]
    .into();
    let input = SmallVecEnumMap::default();
    let feature = ServerDeviceFeature::new(
      0,
      "Vibrate".to_owned(),
      Uuid::new_v4(),
      None,
      None,
      output,
      input,
    );
    let mut features = BTreeMap::new();
    features.insert(0, feature);
    ServerDeviceAttributes::new(&features)
  }

  #[test]
  fn legacy_vibrate_index_equal_to_feature_count_returns_error() {
    let attrs = attrs_with_one_vibrate_feature();
    let msg = VibrateCmdV1::new(0, vec![VibrateSubcommandV1::new(1, 0.5)]);

    let result = CheckedOutputVecCmdV4::try_from_device_attributes(msg, &attrs);

    assert_eq!(
      result.unwrap_err(),
      ButtplugError::from(ButtplugDeviceError::DeviceFeatureIndexError(1, 1))
    );
  }
}
