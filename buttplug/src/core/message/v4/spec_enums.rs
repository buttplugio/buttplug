// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    ButtplugClientMessageV0, ButtplugClientMessageV1, ButtplugClientMessageV2, ButtplugClientMessageV3, ButtplugClientMessageVariant, ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, ButtplugServerMessageV3, DeviceRemovedV0, ErrorV0, LegacyDeviceAttributes, OkV0, PingV0, RawReadCmdV2, RawReadingV2, RawSubscribeCmdV2, RawUnsubscribeCmdV2, RawWriteCmdV2, RequestDeviceListV0, RequestServerInfoV1, ScanningFinishedV0, ServerInfoV2, StartScanningV0, StopAllDevicesV0, StopDeviceCmdV0, StopScanningV0, TryFromClientMessage, TryFromDeviceAttributes
  },
};
use std::collections::HashMap;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{
  level_cmd::InternalLevelCmdV4, DeviceAddedV4, DeviceListV4, LevelCmdV4, LinearCmdV4, SensorReadCmdV4, SensorReadingV4, SensorSubscribeCmdV4, SensorUnsubscribeCmdV4
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
  // Sensor commands
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
  // Raw commands
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

/// An InternalClientMessage has had its contents verified and should need no further internal error
/// checking. Processing may still return errors, but should be due to system state, not message
/// contents.
/// 
/// There should only be one version of InternalClientMessage in the library, matching the latest
/// version of the message spec. For any messages that don't require error checking, their regular
/// struct can be used as an enum parameter. Any messages requiring error checking or validation
/// will have an alternate Internal[x] form that they will need to be cast as.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  FromSpecificButtplugMessage,
)]
pub enum ButtplugInternalClientMessageV4 {
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
  LevelCmd(InternalLevelCmdV4),
  LinearCmd(LinearCmdV4),
  // Sensor commands
  SensorReadCmd(SensorReadCmdV4),
  SensorSubscribeCmd(SensorSubscribeCmdV4),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV4),
  // Raw commands
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
}

impl TryFromClientMessage<ButtplugClientMessageV4> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(value: ButtplugClientMessageV4, feature_map: &HashMap<u32, LegacyDeviceAttributes>) -> Result<Self, ButtplugError> {
    match value {
      // Messages that don't need checking
      ButtplugClientMessageV4::RequestServerInfo(m) => Ok(ButtplugInternalClientMessageV4::RequestServerInfo(m)),
      ButtplugClientMessageV4::Ping(m) => Ok(ButtplugInternalClientMessageV4::Ping(m)),
      ButtplugClientMessageV4::StartScanning(m) => Ok(ButtplugInternalClientMessageV4::StartScanning(m)),
      ButtplugClientMessageV4::StopScanning(m) => Ok(ButtplugInternalClientMessageV4::StopScanning(m)),
      ButtplugClientMessageV4::RequestDeviceList(m) => Ok(ButtplugInternalClientMessageV4::RequestDeviceList(m)),
      ButtplugClientMessageV4::StopAllDevices(m) => Ok(ButtplugInternalClientMessageV4::StopAllDevices(m)),

      // Messages that need device index checking
      ButtplugClientMessageV4::StopDeviceCmd(m) => {
        if feature_map.get(&m.device_index()).is_some() {
          Ok(ButtplugInternalClientMessageV4::StopDeviceCmd(m))
        } else {
          Err(ButtplugError::from(ButtplugDeviceError::DeviceNotAvailable(m.device_index())))
        }
      }

      // Message that need device index and feature checking
      ButtplugClientMessageV4::LevelCmd(m) => {
        if let Some(features) = feature_map.get(&m.device_index()) {
          Ok(ButtplugInternalClientMessageV4::LevelCmd(InternalLevelCmdV4::try_from_device_attributes(m, features)?))
        } else {
          Err(ButtplugError::from(ButtplugDeviceError::DeviceNotAvailable(m.device_index())))
        }
      }
      ButtplugClientMessageV4::LinearCmd(m) => Ok(ButtplugInternalClientMessageV4::LinearCmd(m)),
      ButtplugClientMessageV4::SensorReadCmd(m) => Ok(ButtplugInternalClientMessageV4::SensorReadCmd(m)),
      ButtplugClientMessageV4::SensorSubscribeCmd(m) => Ok(ButtplugInternalClientMessageV4::SensorSubscribeCmd(m)),
      ButtplugClientMessageV4::SensorUnsubscribeCmd(m) => Ok(ButtplugInternalClientMessageV4::SensorUnsubscribeCmd(m)),

      // Message that need device index and hardware endpoint checking
      ButtplugClientMessageV4::RawWriteCmd(m) => Ok(ButtplugInternalClientMessageV4::RawWriteCmd(m)),
      ButtplugClientMessageV4::RawReadCmd(m) => Ok(ButtplugInternalClientMessageV4::RawReadCmd(m)),
      ButtplugClientMessageV4::RawSubscribeCmd(m) => Ok(ButtplugInternalClientMessageV4::RawSubscribeCmd(m)),
      ButtplugClientMessageV4::RawUnsubscribeCmd(m) => Ok(ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m)),
    }
  }
}

// For v3 to v4, all deprecations should be treated as conversions, but will require current
// connected device state, meaning they'll need to be implemented where they can also access the
// device manager.
impl TryFrom<ButtplugClientMessageV3> for ButtplugInternalClientMessageV4 {
  type Error = ButtplugMessageError;

  fn try_from(value: ButtplugClientMessageV3) -> Result<Self, Self::Error> {
    match value {
      ButtplugClientMessageV3::Ping(m) => Ok(ButtplugInternalClientMessageV4::Ping(m.clone())),
      ButtplugClientMessageV3::RequestServerInfo(m) => {
        Ok(ButtplugInternalClientMessageV4::RequestServerInfo(m.clone()))
      }
      ButtplugClientMessageV3::StartScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StartScanning(m.clone()))
      }
      ButtplugClientMessageV3::StopScanning(m) => {
        Ok(ButtplugInternalClientMessageV4::StopScanning(m.clone()))
      }
      ButtplugClientMessageV3::RequestDeviceList(m) => {
        Ok(ButtplugInternalClientMessageV4::RequestDeviceList(m.clone()))
      }
      ButtplugClientMessageV3::StopAllDevices(m) => {
        Ok(ButtplugInternalClientMessageV4::StopAllDevices(m.clone()))
      }
      ButtplugClientMessageV3::StopDeviceCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::StopDeviceCmd(m.clone()))
      }
      ButtplugClientMessageV3::RawReadCmd(m) => Ok(ButtplugInternalClientMessageV4::RawReadCmd(m)),
      ButtplugClientMessageV3::RawWriteCmd(m) => Ok(ButtplugInternalClientMessageV4::RawWriteCmd(m)),
      ButtplugClientMessageV3::RawSubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawSubscribeCmd(m))
      }
      ButtplugClientMessageV3::RawUnsubscribeCmd(m) => {
        Ok(ButtplugInternalClientMessageV4::RawUnsubscribeCmd(m))
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

impl TryFromClientMessage<ButtplugClientMessageVariant> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageVariant,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    let id = msg.id();
    let mut converted_msg = match msg {
      ButtplugClientMessageVariant::V0(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V1(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V2(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V3(m) => Self::try_from_client_message(m, features),
      ButtplugClientMessageVariant::V4(m) => Self::try_from_client_message(m, features)
    }?;
    // Always make sure the ID is set after conversion
    converted_msg.set_id(id);
    Ok(converted_msg)
  }
}

impl TryFromClientMessage<ButtplugClientMessageV0> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV0,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // All v0 messages can be converted to v1 messages.
    Self::try_from_client_message(ButtplugClientMessageV1::from(msg), features)
  }
}

fn check_device_index_and_convert<T, U>(msg: T, features: &HashMap<u32, LegacyDeviceAttributes>) -> Result<U, ButtplugError> where T: ButtplugDeviceMessage, U: TryFromDeviceAttributes<T> {
  // Vorze and RotateCmd are equivalent, so this is an ok conversion.
  if let Some(attrs) = features.get(&msg.device_index()) {
    Ok(U::try_from_device_attributes(msg.clone(), attrs)?.into())
  } else {
    Err(ButtplugError::from(ButtplugDeviceError::DeviceNotAvailable(msg.device_index())))
  }
}

impl TryFromClientMessage<ButtplugClientMessageV1> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV1,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    // Instead of converting to v2 message attributes then to v4 device features, we move directly
    // from v0 command messages to v4 device features here. There's no reason to do the middle step.
    match msg {
      ButtplugClientMessageV1::VorzeA10CycloneCmd(m) => {
        // Vorze and RotateCmd are equivalent, so this is an ok conversion.
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV1::SingleMotorVibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV2::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV2> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV2,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v2 specific queries to v3 generic sensor queries
      ButtplugClientMessageV2::BatteryLevelCmd(m) => {
        Ok(check_device_index_and_convert::<_, SensorReadCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV2::RSSILevelCmd(m) => {
        Ok(check_device_index_and_convert::<_, SensorReadCmdV4>(m, features)?.into())
      }
      // Convert VibrateCmd to a ScalarCmd command
      ButtplugClientMessageV2::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      _ => Self::try_from_client_message(ButtplugClientMessageV3::try_from(msg)?, features),
    }
  }
}

impl TryFromClientMessage<ButtplugClientMessageV3> for ButtplugInternalClientMessageV4 {
  fn try_from_client_message(
    msg: ButtplugClientMessageV3,
    features: &HashMap<u32, LegacyDeviceAttributes>,
  ) -> Result<Self, ButtplugError> {
    match msg {
      // Convert v1/v2 message attribute commands into device feature commands
      ButtplugClientMessageV3::VibrateCmd(m) => {
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::ScalarCmd(m) => {
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::RotateCmd(m) => {
        Ok(check_device_index_and_convert::<_, InternalLevelCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::LinearCmd(m) => {
        Ok(check_device_index_and_convert::<_, LinearCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorReadCmd(m) => {
        Ok(check_device_index_and_convert::<_, SensorReadCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorSubscribeCmd(m) => {
        Ok(check_device_index_and_convert::<_, SensorSubscribeCmdV4>(m, features)?.into())
      }
      ButtplugClientMessageV3::SensorUnsubscribeCmd(m) => {
        Ok(check_device_index_and_convert::<_, SensorUnsubscribeCmdV4>(m, features)?.into())
      }
      _ => ButtplugInternalClientMessageV4::try_from(msg).map_err(|e: ButtplugMessageError| e.into()),
    }
  }
}
