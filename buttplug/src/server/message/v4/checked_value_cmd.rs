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
      ButtplugDeviceMessage,
      ButtplugMessage,
      ButtplugMessageFinalizer,
      ButtplugMessageValidator,
      FeatureType,
      ValueCmdV4,
      ValueSubcommandV4,
    },
  },
  server::message::{
    v0::{SingleMotorVibrateCmdV0, VorzeA10CycloneCmdV0},
    v1::{RotateCmdV1, VibrateCmdV1},
    v3::ScalarCmdV3,
    ButtplugDeviceMessageType,
    ServerDeviceAttributes,
    TryFromDeviceAttributes,
  },
};
use getset::{CopyGetters, Getters};
use uuid::Uuid;

#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct CheckedValueSubcommandV4 {
  feature_index: u32,
  level: i32,
  feature_id: Uuid,
}

impl CheckedValueSubcommandV4 {
  pub fn new(feature_index: u32, level: i32, feature_id: Uuid) -> Self {
    Self {
      feature_index,
      level,
      feature_id,
    }
  }
}

impl From<CheckedValueSubcommandV4> for ValueSubcommandV4 {
  fn from(value: CheckedValueSubcommandV4) -> Self {
    ValueSubcommandV4::new(value.feature_index(), value.level)
  }
}

impl TryFromDeviceAttributes<&ValueSubcommandV4> for CheckedValueSubcommandV4 {
  fn try_from_device_attributes(
    subcommand: &ValueSubcommandV4,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, ButtplugError> {
    let features = attrs.features();
    // Since we have the feature info already, check limit and unpack into step range when creating
    // If this message isn't the result of an upgrade from another older message, we won't have set our feature yet.
    let feature_id = if let Some(feature) = features.get(subcommand.feature_index() as usize) {
      *feature.id()
    } else {
      return Err(ButtplugError::from(
        ButtplugDeviceError::DeviceFeatureIndexError(
          features.len() as u32,
          subcommand.feature_index(),
        ),
      ));
    };

    let feature = features
      .iter()
      .find(|x| *x.id() == feature_id)
      .expect("Already checked existence or created.");
    let level = subcommand.level();
    // Check to make sure the feature has an actuator that handles LevelCmd
    if let Some(actuator) = feature.actuator() {
      // Check to make sure the level is within the range of the feature.
      if actuator
        .messages()
        .contains(&crate::core::message::ButtplugActuatorFeatureMessageType::ValueCmd)
      {
        // Currently, rotate with direction is the only actuator type that can take negative values.
        if *feature.feature_type() == FeatureType::RotateWithDirection
          && !actuator.step_limit().contains(&level.unsigned_abs())
        {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceStepRangeError(
              *actuator.step_limit().end(),
              level.unsigned_abs(),
            ),
          ))
        } else if level < 0 {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceStepRangeError(
              *actuator.step_limit().end(),
              level.unsigned_abs(),
            ),
          ))
        } else if !actuator.step_limit().contains(&level.unsigned_abs()) {
          Err(ButtplugError::from(
            ButtplugDeviceError::DeviceStepRangeError(
              *actuator.step_limit().end(),
              level.unsigned_abs(),
            ),
          ))
        } else {
          Ok(Self {
            feature_id,
            level, //*actuator.step_limit().start() as i32 + level,
            feature_index: subcommand.feature_index(),
          })
        }
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::ValueCmd.to_string()),
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::MessageNotSupported(ButtplugDeviceMessageType::ValueCmd.to_string()),
      ))
    }
  }
}

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
pub struct CheckedValueCmdV4 {
  #[getset(get_copy = "pub")]
  id: u32,
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[getset(get = "pub")]
  levels: Vec<CheckedValueSubcommandV4>,
}

impl From<CheckedValueCmdV4> for ValueCmdV4 {
  fn from(value: CheckedValueCmdV4) -> Self {
    ValueCmdV4::new(
      value.device_index(),
      value
        .levels()
        .iter()
        .map(|x| ValueSubcommandV4::from(x.clone()))
        .collect(),
    )
  }
}

impl CheckedValueCmdV4 {
  pub fn new(id: u32, device_index: u32, levels: &Vec<CheckedValueSubcommandV4>) -> Self {
    Self {
      id,
      device_index,
      levels: levels.clone(),
    }
  }
}

impl ButtplugMessageValidator for CheckedValueCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<ValueCmdV4> for CheckedValueCmdV4 {
  fn try_from_device_attributes(
    msg: ValueCmdV4,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let levels: Result<Vec<CheckedValueSubcommandV4>, ButtplugError> = msg
      .levels()
      .iter()
      .map(|x| CheckedValueSubcommandV4::try_from_device_attributes(x, features))
      .collect();
    Ok(Self {
      id: msg.id(),
      device_index: msg.device_index(),
      levels: levels?,
    })
  }
}

impl TryFromDeviceAttributes<VorzeA10CycloneCmdV0> for CheckedValueCmdV4 {
  fn try_from_device_attributes(
    msg: VorzeA10CycloneCmdV0,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<ValueSubcommandV4> = features
      .features()
      .iter()
      .enumerate()
      .filter(|(_, feature)| *feature.feature_type() == FeatureType::RotateWithDirection)
      .map(|(index, feature)| {
        ValueSubcommandV4::new(
          index as u32,
          (((msg.speed() as f64 / 99f64).ceil() * (if msg.clockwise() { 1f64 } else { -1f64 }))
            * *feature.actuator().as_ref().unwrap().step_range().end() as f64)
            .ceil() as i32,
        )
      })
      .collect();

    CheckedValueCmdV4::try_from_device_attributes(
      ValueCmdV4::new(msg.device_index(), cmds),
      features,
    )
  }
}

impl TryFromDeviceAttributes<SingleMotorVibrateCmdV0> for CheckedValueCmdV4 {
  // For VibrateCmd, just take everything out of V2's VibrateCmd and make a command.
  fn try_from_device_attributes(
    msg: SingleMotorVibrateCmdV0,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<CheckedValueSubcommandV4> = features
      .features()
      .iter()
      .enumerate()
      .filter(|(_, feature)| *feature.feature_type() == FeatureType::Vibrate)
      .map(|(index, feature)| {
        CheckedValueSubcommandV4::new(
          index as u32,
          (msg.speed() * *feature.actuator().as_ref().unwrap().step_range().end() as f64).ceil()
            as i32,
          *feature.id(),
        )
      })
      .collect();

    Ok(CheckedValueCmdV4::new(msg.id(), msg.device_index(), &cmds))
  }
}

impl TryFromDeviceAttributes<VibrateCmdV1> for CheckedValueCmdV4 {
  // VibrateCmd only exists up through Message Spec v2. We can assume that, if we're receiving it,
  // we can just use the V2 spec client device attributes for it. If this was sent on a V1 protocol,
  // it'll still have all the same features.
  //
  // Due to specs v1/2 using feature counts instead of per-feature objects, we calculate our
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

    let mut cmds: Vec<CheckedValueSubcommandV4> = vec![];
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
        .find(|(_, f)| *f.id() == *feature.id())
        .expect("Already checked existence")
        .0;
      let actuator =
        feature
          .actuator()
          .as_ref()
          .ok_or(ButtplugDeviceError::DeviceConfigurationError(
            "Device configuration does not have Vibrate actuator available.".to_owned(),
          ))?;
      cmds.push(CheckedValueSubcommandV4::new(
        idx as u32,
        (vibrate_cmd.speed() * *actuator.step_range().end() as f64).ceil() as i32,
        *feature.id(),
      ))
    }

    Ok(CheckedValueCmdV4::new(msg.id(), msg.device_index(), &cmds))
  }
}

impl TryFromDeviceAttributes<ScalarCmdV3> for CheckedValueCmdV4 {
  // ScalarCmd only came in with V3, so we can just use the V3 device attributes.
  fn try_from_device_attributes(
    msg: ScalarCmdV3,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedValueSubcommandV4> = vec![];
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
            ButtplugDeviceMessageType::ScalarCmd.to_string(),
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
        .find(|(_, f)| *f.id() == *feature.feature().id())
        .expect("Already proved existence")
        .0 as u32;
      let actuator = feature
        .feature()
        .actuator()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("ScalarCmdV3".to_owned()),
        ))?;
      cmds.push(CheckedValueSubcommandV4::new(
        idx,
        (cmd.scalar() * *actuator.step_range().end() as f64).ceil() as i32,
        *feature.feature.id(),
      ));
    }
    Ok(CheckedValueCmdV4::new(msg.id(), msg.device_index(), &cmds))
  }
}

impl TryFromDeviceAttributes<RotateCmdV1> for CheckedValueCmdV4 {
  // RotateCmd exists up through Message Spec v3. We can assume that, if we're receiving it, we can
  // just use the V3 spec client device attributes for it. If this was sent on a V1/V2 protocol,
  // it'll still have all the same features.
  fn try_from_device_attributes(
    msg: RotateCmdV1,
    attrs: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let mut cmds: Vec<CheckedValueSubcommandV4> = vec![];
    for cmd in msg.rotations() {
      let rotate_attrs = attrs
        .attrs_v3()
        .rotate_cmd()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::MessageNotSupported(
            ButtplugDeviceMessageType::RotateCmd.to_string(),
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
        .find(|(_, f)| *f.id() == *feature.feature().id())
        .expect("Already proved existence")
        .0 as u32;
      let actuator = feature
        .feature()
        .actuator()
        .as_ref()
        .ok_or(ButtplugError::from(
          ButtplugDeviceError::DeviceNoActuatorError("RotateCmdV1".to_owned()),
        ))?;
      cmds.push(CheckedValueSubcommandV4::new(
        idx,
        (cmd.speed()
          * *actuator.step_range().end() as f64
          * (if cmd.clockwise() { 1f64 } else { -1f64 }))
        .ceil() as i32,
        *feature.feature().id(),
      ));
    }
    Ok(CheckedValueCmdV4::new(msg.id(), msg.device_index(), &cmds))
  }
}
