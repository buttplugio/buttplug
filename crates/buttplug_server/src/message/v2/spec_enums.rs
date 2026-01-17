// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  StopAllDevicesV0,
  StopDeviceCmdV0,
  v1::{
    ButtplugClientMessageV1,
    ButtplugServerMessageV1,
    LinearCmdV1,
    RequestServerInfoV1,
    RotateCmdV1,
    VibrateCmdV1,
  },
};
use buttplug_core::{
  errors::{ButtplugError, ButtplugMessageError},
  message::{
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    DeviceRemovedV0,
    ErrorV0,
    OkV0,
    PingV0,
    RequestDeviceListV0,
    ScanningFinishedV0,
    StartScanningV0,
    StopScanningV0,
  },
};
use serde::{Deserialize, Serialize};

use super::{BatteryLevelCmdV2, BatteryLevelReadingV2, DeviceAddedV2, DeviceListV2, ServerInfoV2};

/// Represents all client-to-server messages in v2 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, derive_more::From, Serialize, Deserialize)]
pub enum ButtplugClientMessageV2 {
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
  // Sensor commands
  BatteryLevelCmd(BatteryLevelCmdV2),
}

impl ButtplugMessage for ButtplugClientMessageV2 {
  fn id(&self) -> u32 {
    match self {
      ButtplugClientMessageV2::RequestServerInfo(msg) => msg.id(),
      ButtplugClientMessageV2::Ping(msg) => msg.id(),
      ButtplugClientMessageV2::StartScanning(msg) => msg.id(),
      ButtplugClientMessageV2::StopScanning(msg) => msg.id(),
      ButtplugClientMessageV2::RequestDeviceList(msg) => msg.id(),
      ButtplugClientMessageV2::StopAllDevices(msg) => msg.id(),
      ButtplugClientMessageV2::VibrateCmd(msg) => msg.id(),
      ButtplugClientMessageV2::LinearCmd(msg) => msg.id(),
      ButtplugClientMessageV2::RotateCmd(msg) => msg.id(),
      ButtplugClientMessageV2::StopDeviceCmd(msg) => msg.id(),
      ButtplugClientMessageV2::BatteryLevelCmd(msg) => msg.id(),
    }
  }
  fn set_id(&mut self, id: u32) {
    match self {
      ButtplugClientMessageV2::RequestServerInfo(msg) => msg.set_id(id),
      ButtplugClientMessageV2::Ping(msg) => msg.set_id(id),
      ButtplugClientMessageV2::StartScanning(msg) => msg.set_id(id),
      ButtplugClientMessageV2::StopScanning(msg) => msg.set_id(id),
      ButtplugClientMessageV2::RequestDeviceList(msg) => msg.set_id(id),
      ButtplugClientMessageV2::StopAllDevices(msg) => msg.set_id(id),
      ButtplugClientMessageV2::VibrateCmd(msg) => msg.set_id(id),
      ButtplugClientMessageV2::LinearCmd(msg) => msg.set_id(id),
      ButtplugClientMessageV2::RotateCmd(msg) => msg.set_id(id),
      ButtplugClientMessageV2::StopDeviceCmd(msg) => msg.set_id(id),
      ButtplugClientMessageV2::BatteryLevelCmd(msg) => msg.set_id(id),
    }
  }
}

impl ButtplugMessageFinalizer for ButtplugClientMessageV2 {
}

impl ButtplugMessageValidator for ButtplugClientMessageV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugClientMessageV2::RequestServerInfo(msg) => msg.is_valid(),
      ButtplugClientMessageV2::Ping(msg) => msg.is_valid(),
      ButtplugClientMessageV2::StartScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV2::StopScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV2::RequestDeviceList(msg) => msg.is_valid(),
      ButtplugClientMessageV2::StopAllDevices(msg) => msg.is_valid(),
      ButtplugClientMessageV2::VibrateCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV2::LinearCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV2::RotateCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV2::StopDeviceCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV2::BatteryLevelCmd(msg) => msg.is_valid(),
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
        Err(ButtplugMessageError::MessageConversionError("FleshlightLaunchFW12Cmd is not implemented. Please update the client software to use a newer command".to_owned()))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {value:?} to current message spec while lacking state."
      ))),
    }
  }
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, derive_more::From, Serialize, Deserialize)]
pub enum ButtplugServerMessageV2 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV2),
  // Device enumeration messages
  DeviceList(DeviceListV2),
  DeviceAdded(DeviceAddedV2),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
  // Sensor commands
  BatteryLevelReading(BatteryLevelReadingV2),
}

impl ButtplugMessage for ButtplugServerMessageV2 {
  fn id(&self) -> u32 {
    match self {
      ButtplugServerMessageV2::Ok(msg) => msg.id(),
      ButtplugServerMessageV2::Error(msg) => msg.id(),
      ButtplugServerMessageV2::ServerInfo(msg) => msg.id(),
      ButtplugServerMessageV2::DeviceList(msg) => msg.id(),
      ButtplugServerMessageV2::DeviceAdded(msg) => msg.id(),
      ButtplugServerMessageV2::DeviceRemoved(msg) => msg.id(),
      ButtplugServerMessageV2::ScanningFinished(msg) => msg.id(),
      ButtplugServerMessageV2::BatteryLevelReading(msg) => msg.id(),
    }
  }
  fn set_id(&mut self, id: u32) {
    match self {
      ButtplugServerMessageV2::Ok(msg) => msg.set_id(id),
      ButtplugServerMessageV2::Error(msg) => msg.set_id(id),
      ButtplugServerMessageV2::ServerInfo(msg) => msg.set_id(id),
      ButtplugServerMessageV2::DeviceList(msg) => msg.set_id(id),
      ButtplugServerMessageV2::DeviceAdded(msg) => msg.set_id(id),
      ButtplugServerMessageV2::DeviceRemoved(msg) => msg.set_id(id),
      ButtplugServerMessageV2::ScanningFinished(msg) => msg.set_id(id),
      ButtplugServerMessageV2::BatteryLevelReading(msg) => msg.set_id(id),
    }
  }
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV2 {
}

impl ButtplugMessageValidator for ButtplugServerMessageV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugServerMessageV2::Ok(msg) => msg.is_valid(),
      ButtplugServerMessageV2::Error(msg) => msg.is_valid(),
      ButtplugServerMessageV2::ServerInfo(msg) => msg.is_valid(),
      ButtplugServerMessageV2::DeviceList(msg) => msg.is_valid(),
      ButtplugServerMessageV2::DeviceAdded(msg) => msg.is_valid(),
      ButtplugServerMessageV2::DeviceRemoved(msg) => msg.is_valid(),
      ButtplugServerMessageV2::ScanningFinished(msg) => msg.is_valid(),
      ButtplugServerMessageV2::BatteryLevelReading(msg) => msg.is_valid(),
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
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ButtplugDeviceMessageNameV2 {
  LinearCmd,
  RotateCmd,
  StopDeviceCmd,
  VibrateCmd,
  BatteryLevelCmd,
}
