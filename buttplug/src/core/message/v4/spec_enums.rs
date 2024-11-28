// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{errors::ButtplugError, message::{
  ButtplugClientMessageV0, ButtplugClientMessageV1, ButtplugClientMessageV2, ButtplugClientMessageV3, ButtplugClientMessageVariant, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, ButtplugServerMessageV3, DeviceRemovedV0, ErrorV0, LegacyDeviceAttributes, OkV0, PingV0, RawReadCmdV2, RawReadingV2, RawSubscribeCmdV2, RawUnsubscribeCmdV2, RawWriteCmdV2, RequestDeviceListV0, RequestServerInfoV1, ScanningFinishedV0, ServerInfoV2, StartScanningV0, StopAllDevicesV0, StopDeviceCmdV0, StopScanningV0, TryFromClientMessage, TryFromDeviceAttributes
}};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{
  DeviceAddedV4,
  DeviceListV4,
  LinearCmdV4,
  LevelCmdV4,
  SensorReadCmdV4,
  SensorReadingV4,
  SensorSubscribeCmdV4,
  SensorUnsubscribeCmdV4,
};

/// Represents all client-to-server messages in v3 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  LevelCmd(LevelCmdV4),
  LinearCmd(LinearCmdV4),
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
  // Sensor commands
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
}

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

/// Represents all server-to-client messages in v3 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageValidator, FromSpecificButtplugMessage,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV4 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV4),
  DeviceAdded(DeviceAddedV4),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Generic commands
  RawReading(RawReadingV2),
  // Sensor commands
  SensorReading(SensorReadingV4),
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV4 {
  fn finalize(&mut self) {
    match self {
      ButtplugServerMessageV4::DeviceAdded(da) => da.finalize(),
      ButtplugServerMessageV4::DeviceList(dl) => dl.finalize(),
      _ => (),
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

impl TryFromClientMessage<ButtplugClientMessageVariant> for ButtplugClientMessageV4 {
  fn try_from_client_message(msg: ButtplugClientMessageVariant, features: &Option<LegacyDeviceAttributes>) -> Result<Self, crate::core::errors::ButtplugError> {
    let id = msg.id();
    let mut converted_msg = match msg {
      ButtplugClientMessageVariant::V0(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V1(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V2(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V3(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V4(m) => Ok(m)
    }?;
    // Always make sure the ID is set after conversion
    converted_msg.set_id(id);
    Ok(converted_msg)
  }
}

impl TryFromClientMessage<ButtplugClientMessageV0> for ButtplugClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV0,
    features: &Option<LegacyDeviceAttributes>
  ) -> Result<Self, ButtplugError> {
    // All v0 messages can be converted to v1 messages.
    Self::try_from_client_message(ButtplugClientMessageV1::from(msg), features)
  }
}

impl TryFromClientMessage<ButtplugClientMessageV1> for ButtplugClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV1,
    features: &Option<LegacyDeviceAttributes>
  ) -> Result<Self, ButtplugError> {
    // Instead of converting to v2 message attributes then to v4 device features, we move directly
    // from v0 command messages to v4 device features here. There's no reason to do the middle step.
    if let Some(device_features) = &features {
      match msg {
        ButtplugClientMessageV1::VorzeA10CycloneCmd(m)  => {
          // Vorze and RotateCmd are equivalent, so this is an ok conversion.
          Ok(LevelCmdV4::try_from_device_attributes(m, device_features)?.into())
        }
        ButtplugClientMessageV1::SingleMotorVibrateCmd(m) => {
          // Vorze and RotateCmd are equivalent, so this is an ok conversion.
          Ok(LevelCmdV4::try_from_device_attributes(m, device_features)?.into())
        }
        _ => Self::try_from_client_message(ButtplugClientMessageV2::try_from(msg)?, features),
      }  
    } else {
      Self::try_from_client_message(ButtplugClientMessageV2::try_from(msg)?, features)
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV2> for ButtplugClientMessageV4 {
  fn try_from_client_message(msg: ButtplugClientMessageV2, features: &Option<LegacyDeviceAttributes>) -> Result<Self, ButtplugError> {
    if let Some(device_features) = features {
      match msg {
        // Convert v2 specific queries to v3 generic sensor queries
        ButtplugClientMessageV2::BatteryLevelCmd(m) => {
          Ok(SensorReadCmdV4::try_from_device_attributes(m, device_features)?.into())
        }
        ButtplugClientMessageV2::RSSILevelCmd(m) => {
          Ok(SensorReadCmdV4::try_from_device_attributes(m, device_features)?.into())
        }
        // Convert VibrateCmd to a ScalarCmd command
        ButtplugClientMessageV2::VibrateCmd(m) => {
          Ok(LevelCmdV4::try_from_device_attributes(m, device_features)?.into())
        }
        _ => Self::try_from_client_message(ButtplugClientMessageV3::try_from(msg)?, features),
      }
    } else {
      Self::try_from_client_message(ButtplugClientMessageV3::try_from(msg)?, features)
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV3> for ButtplugClientMessageV4 {
  fn try_from_client_message(msg: ButtplugClientMessageV3, features: &Option<LegacyDeviceAttributes>) -> Result<Self, ButtplugError> {
    if let Some(features) = features {
      match msg {
        // Convert v1/v2 message attribute commands into device feature commands
        ButtplugClientMessageV3::VibrateCmd(m) =>
          Ok(LevelCmdV4::try_from_device_attributes(m, features)?.into()),
        ButtplugClientMessageV3::ScalarCmd(m) =>
          Ok(LevelCmdV4::try_from_device_attributes(m, features)?.into()),
        ButtplugClientMessageV3::RotateCmd(m) =>
          Ok(LevelCmdV4::try_from_device_attributes(m, features)?.into()),
        ButtplugClientMessageV3::LinearCmd(m) =>
          Ok(LinearCmdV4::try_from_device_attributes(m, features)?.into()),      
        ButtplugClientMessageV3::SensorReadCmd(m) =>
          Ok(SensorReadCmdV4::try_from_device_attributes(m, features)?.into()),
        ButtplugClientMessageV3::SensorSubscribeCmd(m) =>
          Ok(SensorSubscribeCmdV4::try_from_device_attributes(m, features)?.into()),
        ButtplugClientMessageV3::SensorUnsubscribeCmd(m) =>
          Ok(SensorUnsubscribeCmdV4::try_from_device_attributes(m, features)?.into()),
        _ => ButtplugClientMessageV4::try_from(msg).map_err(|e: ButtplugMessageError| e.into()),
      }
    } else {
      ButtplugClientMessageV4::try_from(msg).map_err(|e: ButtplugMessageError| e.into())
    }
  }
}
