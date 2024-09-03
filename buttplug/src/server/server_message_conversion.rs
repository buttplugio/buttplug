// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Message Spec Conversion
//!
//! This module contains code to convert any message from an older spec version up to the current
//! message spec, and then convert any response from the current message spec back down the sending
//! spec. This is handled within the server, as the server is the only portion of Buttplug that
//! needs to handle up/downgrading (the client should never have to care and should only ever talk
//! one version of the spec, preferably the latest). Having this done within the server also allows
//! us to access required state for converting between messages that requires knowledge of ephemeral
//! device structures (i.e. converting from v4 device features to <= v3 message attributes for
//! messages like DeviceAdded).

use std::fmt::Debug;

use super::device::ServerDeviceManager;
use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
  message::{
    self,
    ActuatorType,
    BatteryLevelCmdV2,
    BatteryLevelReadingV2,
    ButtplugClientMessageV0,
    ButtplugClientMessageV1,
    ButtplugClientMessageV2,
    ButtplugClientMessageV3,
    ButtplugClientMessageV4,
    ButtplugClientMessageVariant,
    ButtplugDeviceMessage,
    ButtplugMessage,
    ButtplugMessageSpecVersion,
    ButtplugServerMessageV0,
    ButtplugServerMessageV1,
    ButtplugServerMessageV2,
    ButtplugServerMessageV3,
    ButtplugServerMessageV4,
    ButtplugServerMessageVariant,
    DeviceFeature,
    ErrorV0,
    FeatureType,
    LinearCmdV1,
    LinearCmdV4,
    RSSILevelCmdV2,
    RSSILevelReadingV2,
    RotateCmdV1,
    RotateCmdV4,
    RotationSubcommandV4,
    ScalarCmdV3,
    ScalarCmdV4,
    ScalarSubcommandV4,
    SensorReadCmdV3,
    SensorReadCmdV4,
    SensorReadingV3,
    SensorSubscribeCmdV3,
    SensorSubscribeCmdV4,
    SensorType,
    SensorUnsubscribeCmdV3,
    SensorUnsubscribeCmdV4,
    VectorSubcommandV4,
    VibrateCmdV1,
    VorzeA10CycloneCmdV0,
  },
};

//
// TryFrom Conversion Traits
//
// Trait implementation is universal for structs, and message structs are defined in the
// core::message module. Even so, we include the TryFrom traits for upgrading client/downgrading
// server messages here in order to keep all of our conversion code in the same module.

// For v3 to v4, all deprecations should be treated as conversions, but will require current
// connected device state, meaning they'll need to be implemented where they can also access the
// device manager.
impl TryFrom<ButtplugClientMessageV3> for ButtplugClientMessageV4 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV3) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV3::Ping(m) => Ok(ButtplugClientMessageV4::Ping(m.clone())),
      ButtplugClientMessageV3::RequestServerInfo(m) => {
        Ok(ButtplugClientMessageV4::RequestServerInfo(m.clone()))
      }
      ButtplugClientMessageV3::StartScanning(m) => {
        Ok(ButtplugClientMessageV4::StartScanning(m.clone()))
      }
      ButtplugClientMessageV3::StopScanning(m) => {
        Ok(ButtplugClientMessageV4::StopScanning(m.clone()))
      }
      ButtplugClientMessageV3::RequestDeviceList(m) => {
        Ok(ButtplugClientMessageV4::RequestDeviceList(m.clone()))
      }
      ButtplugClientMessageV3::StopAllDevices(m) => {
        Ok(ButtplugClientMessageV4::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV3::StopDeviceCmd(m) => {
        Ok(ButtplugClientMessageV4::StopDeviceCmd(m.clone()))
      }
      ButtplugClientMessageV3::RawReadCmd(m) => Ok(ButtplugClientMessageV4::RawReadCmd(m)),
      ButtplugClientMessageV3::RawWriteCmd(m) => Ok(ButtplugClientMessageV4::RawWriteCmd(m)),
      ButtplugClientMessageV3::RawSubscribeCmd(m) => {
        Ok(ButtplugClientMessageV4::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV3::RawUnsubscribeCmd(m) => {
        Ok(ButtplugClientMessageV4::RawUnsubscribeCmd(m))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to V4 message spec while lacking state.",
        value
      ))),
    }
  }
}

// For v2 to v3, all deprecations should be treated as conversions, but will require current
// connected device state, meaning they'll need to be implemented where they can also access the
// device manager.
impl TryFrom<ButtplugClientMessageV2> for ButtplugClientMessageV3 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV2) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV2::Ping(m) => Ok(ButtplugClientMessageV3::Ping(m.clone())),
      ButtplugClientMessageV2::RequestServerInfo(m) => {
        Ok(ButtplugClientMessageV3::RequestServerInfo(m.clone()))
      }
      ButtplugClientMessageV2::StartScanning(m) => {
        Ok(ButtplugClientMessageV3::StartScanning(m.clone()))
      }
      ButtplugClientMessageV2::StopScanning(m) => {
        Ok(ButtplugClientMessageV3::StopScanning(m.clone()))
      }
      ButtplugClientMessageV2::RequestDeviceList(m) => {
        Ok(ButtplugClientMessageV3::RequestDeviceList(m.clone()))
      }
      ButtplugClientMessageV2::StopAllDevices(m) => {
        Ok(ButtplugClientMessageV3::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV2::StopDeviceCmd(m) => {
        Ok(ButtplugClientMessageV3::StopDeviceCmd(m.clone()))
      }
      // Vibrate was supposed to be phased out in v3 but was left in the allowable message set.
      // Oops.
      ButtplugClientMessageV2::VibrateCmd(m) => Ok(ButtplugClientMessageV3::VibrateCmd(m)),
      ButtplugClientMessageV2::LinearCmd(m) => Ok(ButtplugClientMessageV3::LinearCmd(m)),
      ButtplugClientMessageV2::RotateCmd(m) => Ok(ButtplugClientMessageV3::RotateCmd(m)),
      ButtplugClientMessageV2::RawReadCmd(m) => Ok(ButtplugClientMessageV3::RawReadCmd(m)),
      ButtplugClientMessageV2::RawWriteCmd(m) => Ok(ButtplugClientMessageV3::RawWriteCmd(m)),
      ButtplugClientMessageV2::RawSubscribeCmd(m) => {
        Ok(ButtplugClientMessageV3::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV2::RawUnsubscribeCmd(m) => {
        Ok(ButtplugClientMessageV3::RawUnsubscribeCmd(m))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to V3 message spec while lacking state.",
        value
      ))),
    }
  }
}

// For v1 to v2, several messages were deprecated. Throw errors when trying to convert those.
impl TryFrom<ButtplugClientMessageV1> for ButtplugClientMessageV2 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV1) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV1::Ping(m) => Ok(ButtplugClientMessageV2::Ping(m.clone())),
      ButtplugClientMessageV1::RequestServerInfo(m) => {
        Ok(ButtplugClientMessageV2::RequestServerInfo(m.clone()))
      }
      ButtplugClientMessageV1::StartScanning(m) => {
        Ok(ButtplugClientMessageV2::StartScanning(m.clone()))
      }
      ButtplugClientMessageV1::StopScanning(m) => {
        Ok(ButtplugClientMessageV2::StopScanning(m.clone()))
      }
      ButtplugClientMessageV1::RequestDeviceList(m) => {
        Ok(ButtplugClientMessageV2::RequestDeviceList(m.clone()))
      }
      ButtplugClientMessageV1::StopAllDevices(m) => {
        Ok(ButtplugClientMessageV2::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV1::StopDeviceCmd(m) => {
        Ok(ButtplugClientMessageV2::StopDeviceCmd(m.clone()))
      }
      ButtplugClientMessageV1::VibrateCmd(m) => Ok(ButtplugClientMessageV2::VibrateCmd(m.clone())),
      ButtplugClientMessageV1::LinearCmd(m) => Ok(ButtplugClientMessageV2::LinearCmd(m.clone())),
      ButtplugClientMessageV1::RotateCmd(m) => Ok(ButtplugClientMessageV2::RotateCmd(m.clone())),
      ButtplugClientMessageV1::FleshlightLaunchFW12Cmd(_) => {
        // Direct access to FleshlightLaunchFW12Cmd could cause some devices to break via rapid
        // changes of position/speed. Yes, some Kiiroo devices really *are* that fragile.
        Err(ButtplugMessageError::MessageConversionError("FleshlightLaunchFW12Cmd is not implemented. Please update the client software to use a newer command".to_owned()).into())
      }
      ButtplugClientMessageV1::RequestLog(_) => {
        // Log was a huge security hole, as we'd just send our server logs to whomever asked, which
        // contain all sorts of identifying information. Always return an error here.
        Err(
          ButtplugMessageError::MessageConversionError(
            "RequestLog is no longer allowed by any version of Buttplug.".to_owned(),
          )
          .into(),
        )
      }
      ButtplugClientMessageV1::KiirooCmd(_) => {
        // No device protocol implementation ever worked with KiirooCmd, so no one ever should've
        // used it. We'll just return an error if we ever see it.
        Err(ButtplugMessageError::MessageConversionError("KiirooCmd is not implemented. Please update the client software to use a newer command".to_owned()).into())
      }
      ButtplugClientMessageV1::LovenseCmd(_) => {
        // LovenseCmd allowed users to directly send strings to a Lovense device, which was a Bad
        // Idea. Will always return an error.
        Err(ButtplugMessageError::MessageConversionError("LovenseCmd is not implemented. Please update the client software to use a newer command".to_owned()).into())
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to current message spec while lacking state.",
        value
      ))),
    }
  }
}

// No messages were changed or deprecated before v2, so we can convert all v0 messages to v1.
impl From<ButtplugClientMessageV0> for ButtplugClientMessageV1 {
  fn from(value: ButtplugClientMessageV0) -> Self {
    match value {
      ButtplugClientMessageV0::Ping(m) => ButtplugClientMessageV1::Ping(m),
      ButtplugClientMessageV0::RequestServerInfo(m) => {
        ButtplugClientMessageV1::RequestServerInfo(m)
      }
      ButtplugClientMessageV0::StartScanning(m) => ButtplugClientMessageV1::StartScanning(m),
      ButtplugClientMessageV0::StopScanning(m) => ButtplugClientMessageV1::StopScanning(m),
      ButtplugClientMessageV0::RequestDeviceList(m) => {
        ButtplugClientMessageV1::RequestDeviceList(m)
      }
      ButtplugClientMessageV0::StopAllDevices(m) => ButtplugClientMessageV1::StopAllDevices(m),
      ButtplugClientMessageV0::StopDeviceCmd(m) => ButtplugClientMessageV1::StopDeviceCmd(m),
      ButtplugClientMessageV0::FleshlightLaunchFW12Cmd(m) => {
        ButtplugClientMessageV1::FleshlightLaunchFW12Cmd(m)
      }
      ButtplugClientMessageV0::KiirooCmd(m) => ButtplugClientMessageV1::KiirooCmd(m),
      ButtplugClientMessageV0::LovenseCmd(m) => ButtplugClientMessageV1::LovenseCmd(m),
      ButtplugClientMessageV0::RequestLog(m) => ButtplugClientMessageV1::RequestLog(m),
      ButtplugClientMessageV0::SingleMotorVibrateCmd(m) => {
        ButtplugClientMessageV1::SingleMotorVibrateCmd(m)
      }
      ButtplugClientMessageV0::VorzeA10CycloneCmd(m) => {
        ButtplugClientMessageV1::VorzeA10CycloneCmd(m)
      }
    }
  }
}

impl TryFrom<ButtplugServerMessageV4> for ButtplugServerMessageV3 {
  type Error = ButtplugMessageError;

  fn try_from(
    value: ButtplugServerMessageV4,
  ) -> Result<Self, <ButtplugServerMessageV3 as TryFrom<ButtplugServerMessageV4>>::Error> {
    match value {
      // Direct conversions
      ButtplugServerMessageV4::Ok(m) => Ok(ButtplugServerMessageV3::Ok(m)),
      ButtplugServerMessageV4::Error(m) => Ok(ButtplugServerMessageV3::Error(m)),
      ButtplugServerMessageV4::ServerInfo(m) => Ok(ButtplugServerMessageV3::ServerInfo(m)),
      ButtplugServerMessageV4::DeviceRemoved(m) => Ok(ButtplugServerMessageV3::DeviceRemoved(m)),
      ButtplugServerMessageV4::ScanningFinished(m) => {
        Ok(ButtplugServerMessageV3::ScanningFinished(m))
      }
      ButtplugServerMessageV4::RawReading(m) => Ok(ButtplugServerMessageV3::RawReading(m)),
      ButtplugServerMessageV4::DeviceList(m) => Ok(ButtplugServerMessageV3::DeviceList(m.into())),
      ButtplugServerMessageV4::DeviceAdded(m) => Ok(ButtplugServerMessageV3::DeviceAdded(m.into())),
      // All other messages (SensorReading) requires device manager context.
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to current message spec while lacking state.",
        value
      ))),
    }
  }
}

impl From<ButtplugServerMessageV3> for ButtplugServerMessageV2 {
  fn from(value: ButtplugServerMessageV3) -> Self {
    match value {
      ButtplugServerMessageV3::Ok(m) => ButtplugServerMessageV2::Ok(m),
      ButtplugServerMessageV3::Error(m) => ButtplugServerMessageV2::Error(m),
      ButtplugServerMessageV3::ServerInfo(m) => ButtplugServerMessageV2::ServerInfo(m),
      ButtplugServerMessageV3::DeviceRemoved(m) => ButtplugServerMessageV2::DeviceRemoved(m),
      ButtplugServerMessageV3::ScanningFinished(m) => ButtplugServerMessageV2::ScanningFinished(m),
      ButtplugServerMessageV3::RawReading(m) => ButtplugServerMessageV2::RawReading(m),
      ButtplugServerMessageV3::DeviceAdded(m) => ButtplugServerMessageV2::DeviceAdded(m.into()),
      ButtplugServerMessageV3::DeviceList(m) => ButtplugServerMessageV2::DeviceList(m.into()),
      ButtplugServerMessageV3::SensorReading(_) => ButtplugServerMessageV2::Error(ErrorV0::from(
        ButtplugError::from(ButtplugMessageError::MessageConversionError(
          "SensorReading cannot be converted to Buttplug Message Spec V2".to_owned(),
        )),
      )),
    }
  }
}

impl From<ButtplugServerMessageV2> for ButtplugServerMessageV1 {
  fn from(value: ButtplugServerMessageV2) -> Self {
    match value {
      ButtplugServerMessageV2::Ok(m) => ButtplugServerMessageV1::Ok(m),
      ButtplugServerMessageV2::Error(m) => ButtplugServerMessageV1::Error(m),
      ButtplugServerMessageV2::ServerInfo(m) => ButtplugServerMessageV1::ServerInfo(m.into()),
      ButtplugServerMessageV2::DeviceRemoved(m) => ButtplugServerMessageV1::DeviceRemoved(m),
      ButtplugServerMessageV2::ScanningFinished(m) => ButtplugServerMessageV1::ScanningFinished(m),
      ButtplugServerMessageV2::DeviceAdded(m) => ButtplugServerMessageV1::DeviceAdded(m.into()),
      ButtplugServerMessageV2::DeviceList(m) => ButtplugServerMessageV1::DeviceList(m.into()),
      ButtplugServerMessageV2::BatteryLevelReading(_) => {
        ButtplugServerMessageV1::Error(ErrorV0::from(ButtplugError::from(
          ButtplugMessageError::MessageConversionError(
            "BatteryLevelReading cannot be converted to Buttplug Message Spec V1".to_owned(),
          ),
        )))
      }
      ButtplugServerMessageV2::RSSILevelReading(_) => {
        ButtplugServerMessageV1::Error(ErrorV0::from(ButtplugError::from(
          ButtplugMessageError::MessageConversionError(
            "RSSILevelReading cannot be converted to Buttplug Message Spec V1".to_owned(),
          ),
        )))
      }
      ButtplugServerMessageV2::RawReading(_) => ButtplugServerMessageV1::Error(ErrorV0::from(
        ButtplugError::from(ButtplugMessageError::MessageConversionError(
          "RawReading cannot be converted to Buttplug Message Spec V1".to_owned(),
        )),
      )),
    }
  }
}

impl From<ButtplugServerMessageV1> for ButtplugServerMessageV0 {
  fn from(value: ButtplugServerMessageV1) -> Self {
    match value {
      ButtplugServerMessageV1::Ok(m) => ButtplugServerMessageV0::Ok(m),
      ButtplugServerMessageV1::Error(m) => ButtplugServerMessageV0::Error(m),
      ButtplugServerMessageV1::ServerInfo(m) => ButtplugServerMessageV0::ServerInfo(m.into()),
      ButtplugServerMessageV1::DeviceRemoved(m) => ButtplugServerMessageV0::DeviceRemoved(m),
      ButtplugServerMessageV1::ScanningFinished(m) => ButtplugServerMessageV0::ScanningFinished(m),
      ButtplugServerMessageV1::DeviceAdded(m) => ButtplugServerMessageV0::DeviceAdded(m.into()),
      ButtplugServerMessageV1::DeviceList(m) => ButtplugServerMessageV0::DeviceList(m.into()),
      ButtplugServerMessageV1::Log(_) => ButtplugServerMessageV0::Error(ErrorV0::from(
        ButtplugError::from(ButtplugMessageError::MessageConversionError(
          "For security reasons, Log should never be sent from a Buttplug Server".to_owned(),
        )),
      )),
    }
  }
}

pub struct ButtplugServerMessageConverter {
  original_message: Option<ButtplugClientMessageVariant>,
}

impl ButtplugServerMessageConverter {
  pub fn new(msg: Option<ButtplugClientMessageVariant>) -> Self {
    Self {
      original_message: msg,
    }
  }

  pub fn convert_incoming(
    &self,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    if let Some(msg) = &self.original_message {
      let mut outgoing_msg = match msg {
        ButtplugClientMessageVariant::V0(m) => self.convert_incoming_v0(m, device_manager)?,
        ButtplugClientMessageVariant::V1(m) => self.convert_incoming_v1(m, device_manager)?,
        ButtplugClientMessageVariant::V2(m) => self.convert_incoming_v2(m, device_manager)?,
        ButtplugClientMessageVariant::V3(m) => self.convert_incoming_v3(m, device_manager)?,
        ButtplugClientMessageVariant::V4(m) => m.clone(),
      };
      // Always make sure the ID is set after conversion
      outgoing_msg.set_id(msg.id());
      Ok(outgoing_msg)
    } else {
      Err(
        ButtplugMessageError::MessageConversionError(
          "No incoming message provided for conversion".to_owned(),
        )
        .into(),
      )
    }
  }

  fn convert_incoming_v0(
    &self,
    msg_v0: &ButtplugClientMessageV0,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    // All v0 messages can be converted to v1 messages.
    self.convert_incoming_v1(&msg_v0.clone().into(), device_manager)
  }

  fn convert_incoming_v1(
    &self,
    msg_v1: &ButtplugClientMessageV1,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    // Instead of converting to v2 message attributes then to v4 device features, we move directly
    // from v0 command messages to v4 device features here. There's no reason to do the middle step.
    match msg_v1 {
      ButtplugClientMessageV1::VorzeA10CycloneCmd(m) => {
        // Vorze and RotateCmd are equivalent, so this is an ok conversion.
        self.convert_vorzea10cyclonecmdv0_to_rotatecmdv4(m, device_manager)
      }
      ButtplugClientMessageV1::SingleMotorVibrateCmd(m) => {
        // SingleMotorVibrate is a ScalarCmd w/ Vibrate type for all vibrate functionality.
        self.convert_singlemotorvibratecmdv0_to_scalarcmdv4(m, device_manager)
      }
      _ => self.convert_incoming_v2(&msg_v1.clone().try_into()?, device_manager),
    }
  }

  fn convert_incoming_v2(
    &self,
    msg_v2: &ButtplugClientMessageV2,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    match msg_v2 {
      // Convert v2 specific queries to v3 generic sensor queries
      ButtplugClientMessageV2::BatteryLevelCmd(m) => {
        self.convert_batterylevelcmd_v2_to_sensorreadcmd_v4(m, device_manager)
      }
      ButtplugClientMessageV2::RSSILevelCmd(m) => {
        self.convert_rssilevelcmd_v2_to_sensorreadv4(m, device_manager)
      }
      // Convert VibrateCmd to a ScalarCmd command
      ButtplugClientMessageV2::VibrateCmd(m) => {
        self.convert_vibratecmdv1_to_scalarcmdv4(m, device_manager)
      }
      _ => self.convert_incoming_v3(&msg_v2.clone().try_into()?, device_manager),
    }
  }

  fn convert_incoming_v3(
    &self,
    msg_v3: &ButtplugClientMessageV3,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    match msg_v3 {
      // Convert v1/v2 message attribute commands into device feature commands
      ButtplugClientMessageV3::VibrateCmd(m) => {
        self.convert_vibratecmdv1_to_scalarcmdv4(m, device_manager)
      }
      ButtplugClientMessageV3::ScalarCmd(m) => {
        self.convert_scalarcmdv3_to_scalarcmdv4(m, device_manager)
      }
      ButtplugClientMessageV3::RotateCmd(m) => {
        self.convert_rotatecmdv1_to_scalarcmdv4(m, device_manager)
      }
      ButtplugClientMessageV3::LinearCmd(m) => {
        self.convert_linearcmdv1_to_linearcmdv4(m, device_manager)
      }
      ButtplugClientMessageV3::SensorReadCmd(m) => {
        self.convert_sensorreadv3_to_sensorreadv4(m, device_manager)
      }
      ButtplugClientMessageV3::SensorSubscribeCmd(m) => {
        self.convert_sensorsubscribev3_to_sensorsubcribe4(m, device_manager)
      }
      ButtplugClientMessageV3::SensorUnsubscribeCmd(m) => {
        self.convert_sensorunsubscribev3_to_sensorunsubcribe4(m, device_manager)
      }
      _ => msg_v3
        .clone()
        .try_into()
        .map_err(|e: ButtplugMessageError| e.into()),
    }
  }

  //
  // Incoming Conversion Utility Methods
  //

  fn find_device_features<M, P>(
    &self,
    message: &M,
    device_manager: &ServerDeviceManager,
    criteria: P,
  ) -> Result<Vec<usize>, ButtplugError>
  where
    M: ButtplugDeviceMessage + Debug,
    P: FnMut(&(usize, &DeviceFeature)) -> bool,
  {
    let device_index = message.device_index();

    let device = device_manager
      .devices()
      .get(&device_index)
      .ok_or(ButtplugDeviceError::DeviceNotAvailable(device_index))?;

    let features: Vec<usize> = device
      .definition()
      .features()
      .iter()
      .enumerate()
      .filter(criteria)
      .map(|(index, _)| index)
      .collect();

    if features.is_empty() {
      Err(
        ButtplugDeviceError::ProtocolRequirementError(format!(
          "Device does not have any features that accommodate the following message: {:?}",
          message
        ))
        .into(),
      )
    } else {
      Ok(features)
    }
  }

  fn convert_singlemotorvibratecmdv0_to_scalarcmdv4(
    &self,
    message: &message::SingleMotorVibrateCmdV0,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let vibrate_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        *x.feature_type() == FeatureType::Vibrate
          && x.actuator().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
          })
      })?;

    let cmds: Vec<ScalarSubcommandV4> = vibrate_features
      .iter()
      .map(|x| ScalarSubcommandV4::new(*x as u32, message.speed(), ActuatorType::Vibrate))
      .collect();

    Ok(ScalarCmdV4::new(message.device_index(), cmds).into())
  }

  fn convert_vorzea10cyclonecmdv0_to_rotatecmdv4(
    &self,
    message: &VorzeA10CycloneCmdV0,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let rotate_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        *x.feature_type() == FeatureType::Rotate
          && x.actuator().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugActuatorFeatureMessageType::RotateCmd)
          })
      })?;

    let cmds: Vec<RotationSubcommandV4> = rotate_features
      .iter()
      .map(|x| {
        RotationSubcommandV4::new(
          *x as u32,
          message.speed() as f64 / 99f64,
          message.clockwise(),
        )
      })
      .collect();

    Ok(RotateCmdV4::new(message.device_index(), cmds).into())
  }

  fn convert_batterylevelcmd_v2_to_sensorreadcmd_v4(
    &self,
    message: &BatteryLevelCmdV2,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let battery_features = self.find_device_features(message, device_manager, |(_, x)| {
      *x.feature_type() == FeatureType::Battery
        && x.sensor().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
        })
    })?;

    Ok(
      SensorReadCmdV4::new(
        message.device_index(),
        battery_features[0] as u32,
        SensorType::Battery,
      )
      .into(),
    )
  }

  fn convert_rssilevelcmd_v2_to_sensorreadv4(
    &self,
    message: &RSSILevelCmdV2,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let rssi_features = self.find_device_features(message, device_manager, |(_, x)| {
      *x.feature_type() == FeatureType::RSSI
        && x.sensor().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
        })
    })?;
    Ok(
      SensorReadCmdV4::new(
        message.device_index(),
        rssi_features[0] as u32,
        SensorType::RSSI,
      )
      .into(),
    )
  }

  fn convert_vibratecmdv1_to_scalarcmdv4(
    &self,
    message: &VibrateCmdV1,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let vibrate_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        *x.feature_type() == FeatureType::Vibrate
          && x.actuator().as_ref().is_some_and(|y| {
            y.messages()
              .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
          })
      })?;

    let cmds: Vec<ScalarSubcommandV4> = message
      .speeds()
      .iter()
      .map(|x| {
        ScalarSubcommandV4::new(
          vibrate_features[x.index() as usize] as u32,
          x.speed(),
          ActuatorType::Vibrate,
        )
      })
      .collect();

    Ok(ScalarCmdV4::new(message.device_index(), cmds).into())
  }

  fn convert_scalarcmdv3_to_scalarcmdv4(
    &self,
    message: &ScalarCmdV3,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let scalar_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::ScalarCmd)
        })
      })?;

    let scalars_v4: Vec<ScalarSubcommandV4> = message
      .scalars()
      .iter()
      .map(|x| {
        ScalarSubcommandV4::new(
          scalar_features[x.index() as usize] as u32,
          x.scalar().clone(),
          x.actuator_type().clone(),
        )
      })
      .collect();

    Ok(ScalarCmdV4::new(message.device_index(), scalars_v4).into())
  }

  fn convert_rotatecmdv1_to_scalarcmdv4(
    &self,
    message: &RotateCmdV1,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let rotate_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::RotateCmd)
        })
      })?;

    let cmds: Vec<RotationSubcommandV4> = message
      .rotations()
      .iter()
      .map(|x| {
        RotationSubcommandV4::new(
          rotate_features[x.index() as usize] as u32,
          x.speed(),
          x.clockwise(),
        )
      })
      .collect();

    Ok(RotateCmdV4::new(message.device_index(), cmds).into())
  }

  fn convert_linearcmdv1_to_linearcmdv4(
    &self,
    message: &LinearCmdV1,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let linear_features: Vec<usize> =
      self.find_device_features(message, device_manager, |(_, x)| {
        x.actuator().as_ref().is_some_and(|y| {
          y.messages()
            .contains(&message::ButtplugActuatorFeatureMessageType::LinearCmd)
        })
      })?;

    let cmds: Vec<VectorSubcommandV4> = message
      .vectors()
      .iter()
      .map(|x| {
        VectorSubcommandV4::new(
          linear_features[x.index() as usize] as u32,
          x.duration(),
          x.position(),
        )
      })
      .collect();

    Ok(LinearCmdV4::new(message.device_index(), cmds).into())
  }

  fn convert_sensorreadv3_to_sensorreadv4(
    &self,
    message: &SensorReadCmdV3,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let features = self.find_device_features(message, device_manager, |(_, x)| {
      x.sensor().as_ref().is_some_and(|y| {
        y.messages()
          .contains(&message::ButtplugSensorFeatureMessageType::SensorReadCmd)
      })
    })?;

    let sensor_feature_index = features[*message.sensor_index() as usize] as u32;

    Ok(
      SensorReadCmdV4::new(
        message.device_index(),
        sensor_feature_index,
        *message.sensor_type(),
      )
      .into(),
    )
  }

  fn convert_sensorsubscribev3_to_sensorsubcribe4(
    &self,
    message: &SensorSubscribeCmdV3,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let features = self.find_device_features(message, device_manager, |(_, x)| {
      x.sensor().as_ref().is_some_and(|y| {
        y.messages()
          .contains(&message::ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
      })
    })?;

    let sensor_feature_index = features[*message.sensor_index() as usize] as u32;

    Ok(
      SensorSubscribeCmdV4::new(
        message.device_index(),
        sensor_feature_index,
        *message.sensor_type(),
      )
      .into(),
    )
  }

  fn convert_sensorunsubscribev3_to_sensorunsubcribe4(
    &self,
    message: &SensorUnsubscribeCmdV3,
    device_manager: &ServerDeviceManager,
  ) -> Result<ButtplugClientMessageV4, ButtplugError> {
    let features = self.find_device_features(message, device_manager, |(_, x)| {
      x.sensor().as_ref().is_some_and(|y| {
        y.messages()
          .contains(&message::ButtplugSensorFeatureMessageType::SensorSubscribeCmd)
      })
    })?;

    let sensor_feature_index = features[*message.sensor_index() as usize] as u32;

    Ok(
      SensorUnsubscribeCmdV4::new(
        message.device_index(),
        sensor_feature_index,
        *message.sensor_type(),
      )
      .into(),
    )
  }

  //
  // Outgoing Conversion
  //

  pub fn convert_outgoing(
    &self,
    msg: &ButtplugServerMessageV4,
    version: &ButtplugMessageSpecVersion,
  ) -> Result<ButtplugServerMessageVariant, ButtplugError> {
    let mut outgoing_msg = match version {
      ButtplugMessageSpecVersion::Version0 => {
        ButtplugServerMessageVariant::V0(self.convert_servermessagev4_to_servermessagev0(msg)?)
      }
      ButtplugMessageSpecVersion::Version1 => {
        ButtplugServerMessageVariant::V1(self.convert_servermessagev4_to_servermessagev1(msg)?)
      }
      ButtplugMessageSpecVersion::Version2 => {
        ButtplugServerMessageVariant::V2(self.convert_servermessagev4_to_servermessagev2(msg)?)
      }
      ButtplugMessageSpecVersion::Version3 => {
        ButtplugServerMessageVariant::V3(self.convert_servermessagev4_to_servermessagev3(msg)?)
      }
      ButtplugMessageSpecVersion::Version4 => ButtplugServerMessageVariant::V4(msg.clone()),
    };
    // Always make sure the ID is set after conversion
    outgoing_msg.set_id(msg.id());
    Ok(outgoing_msg)
  }

  fn convert_servermessagev4_to_servermessagev3(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV3, ButtplugError> {
    match msg {
      ButtplugServerMessageV4::SensorReading(m) => {
        let original_msg = self.original_message.as_ref().unwrap();
        if let ButtplugClientMessageVariant::V3(ButtplugClientMessageV3::SensorReadCmd(msg)) =
          &original_msg
        {
          let msg_out = SensorReadingV3::new(
            msg.device_index(),
            *msg.sensor_index(),
            *msg.sensor_type(),
            m.data().clone(),
          );
          Ok(msg_out.into())
        } else {
          Err(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned()).into())
        }
      }
      _ => Ok(msg.clone().try_into()?),
    }
  }

  fn convert_servermessagev4_to_servermessagev2(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV2, ButtplugError> {
    let msg_v3 = self.convert_servermessagev4_to_servermessagev3(msg)?;
    match msg_v3 {
      ButtplugServerMessageV3::SensorReading(m) => {
        let original_msg = self.original_message.as_ref().unwrap();
        // Sensor Reading didn't exist in v2, we only had Battery or RSSI. Therefore we need to
        // context of the original message to make sure this conversion happens correctly.
        if let ButtplugClientMessageVariant::V2(ButtplugClientMessageV2::BatteryLevelCmd(msg)) =
          &original_msg
        {
          Ok(BatteryLevelReadingV2::new(msg.device_index(), m.data()[0] as f64 / 100f64).into())
        } else if let ButtplugClientMessageVariant::V2(ButtplugClientMessageV2::RSSILevelCmd(msg)) =
          &original_msg
        {
          Ok(RSSILevelReadingV2::new(msg.device_index(), m.data()[0]).into())
        } else {
          Err(ButtplugMessageError::UnexpectedMessageType("SensorReading".to_owned()).into())
        }
      }
      _ => Ok(msg_v3.into()),
    }
  }

  fn convert_servermessagev4_to_servermessagev1(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV1, ButtplugError> {
    Ok(self.convert_servermessagev4_to_servermessagev2(msg)?.into())
  }

  fn convert_servermessagev4_to_servermessagev0(
    &self,
    msg: &ButtplugServerMessageV4,
  ) -> Result<ButtplugServerMessageV0, ButtplugError> {
    Ok(self.convert_servermessagev4_to_servermessagev1(msg)?.into())
  }

  // Outgoing Conversion Utility Methods
}
