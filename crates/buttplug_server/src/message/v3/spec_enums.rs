// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v1::{LinearCmdV1, RequestServerInfoV1, RotateCmdV1, VibrateCmdV1},
  v2::{ButtplugClientMessageV2, ButtplugServerMessageV2, ServerInfoV2},
};
use buttplug_core::{
  errors::{ButtplugError, ButtplugMessageError},
  message::{
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    ButtplugServerMessageV4,
    DeviceRemovedV0,
    ErrorV0,
    OkV0,
    PingV0,
    RequestDeviceListV0,
    ScanningFinishedV0,
    StartScanningV0,
    StopAllDevicesV0,
    StopDeviceCmdV0,
    StopScanningV0,
  },
};
use serde::{Deserialize, Serialize};

use super::{
  DeviceAddedV3,
  DeviceListV3,
  ScalarCmdV3,
  SensorReadCmdV3,
  SensorReadingV3,
  SensorSubscribeCmdV3,
  SensorUnsubscribeCmdV3,
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
  Serialize,
  Deserialize,
)]
pub enum ButtplugClientMessageV3 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV1),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  VibrateCmd(VibrateCmdV1),
  LinearCmd(LinearCmdV1),
  RotateCmd(RotateCmdV1),
  StopDeviceCmd(StopDeviceCmdV0),
  ScalarCmd(ScalarCmdV3),
  // Sensor commands
  SensorReadCmd(SensorReadCmdV3),
  SensorSubscribeCmd(SensorSubscribeCmdV3),
  SensorUnsubscribeCmd(SensorUnsubscribeCmdV3),
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
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {value:?} to V3 message spec while lacking state."
      ))),
    }
  }
}

/// Represents all server-to-client messages in v3 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugMessageValidator,
  FromSpecificButtplugMessage,
  Serialize,
  Deserialize,
)]
pub enum ButtplugServerMessageV3 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV3),
  DeviceAdded(DeviceAddedV3),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Sensor commands
  SensorReading(SensorReadingV3),
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV3 {
  fn finalize(&mut self) {
    match self {
      ButtplugServerMessageV3::DeviceAdded(da) => da.finalize(),
      ButtplugServerMessageV3::DeviceList(dl) => dl.finalize(),
      _ => (),
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

impl TryFrom<ButtplugServerMessageV4> for ButtplugServerMessageV3 {
  type Error = ButtplugMessageError;

  fn try_from(
    value: ButtplugServerMessageV4,
  ) -> Result<Self, <ButtplugServerMessageV3 as TryFrom<ButtplugServerMessageV4>>::Error> {
    match value {
      // Direct conversions
      ButtplugServerMessageV4::Ok(m) => Ok(ButtplugServerMessageV3::Ok(m)),
      ButtplugServerMessageV4::Error(m) => Ok(ButtplugServerMessageV3::Error(m)),
      ButtplugServerMessageV4::ServerInfo(m) => Ok(ButtplugServerMessageV3::ServerInfo(m.into())),
      ButtplugServerMessageV4::ScanningFinished(m) => {
        Ok(ButtplugServerMessageV3::ScanningFinished(m))
      }
      ButtplugServerMessageV4::DeviceList(m) => Ok(ButtplugServerMessageV3::DeviceList(m.into())),
      // All other messages (SensorReading) requires device manager context.
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {value:?} to current message spec while lacking state."
      ))),
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ButtplugDeviceMessageNameV3 {
  LinearCmd,
  RotateCmd,
  StopDeviceCmd,
  ScalarCmd,
  SensorReadCmd,
  SensorSubscribeCmd,
  SensorUnsubscribeCmd,
}
