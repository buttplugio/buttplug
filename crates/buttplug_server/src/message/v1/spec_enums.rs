// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::cmp::Ordering;

use crate::message::v0::{
  ButtplugClientMessageV0,
  ButtplugServerMessageV0,
  FleshlightLaunchFW12CmdV0,
  ServerInfoV0,
  SingleMotorVibrateCmdV0,
  VorzeA10CycloneCmdV0,
};
use buttplug_core::{
  errors::ButtplugMessageError,
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
    StopAllDevicesV0,
    StopDeviceCmdV0,
    StopScanningV0,
  },
};
use serde::{Deserialize, Serialize};

use super::{
  DeviceAddedV1,
  DeviceListV1,
  LinearCmdV1,
  RequestServerInfoV1,
  RotateCmdV1,
  VibrateCmdV1,
};

/// Represents all client-to-server messages in v1 of the Buttplug Spec
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
pub enum ButtplugClientMessageV1 {
  // Handshake and server messages
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
  // Deprecated generic commands (not removed until v2)
  SingleMotorVibrateCmd(SingleMotorVibrateCmdV0),
  // Deprecated device specific commands (not removed until v2)
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12CmdV0),
  VorzeA10CycloneCmd(VorzeA10CycloneCmdV0),
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
      ButtplugClientMessageV0::SingleMotorVibrateCmd(m) => {
        ButtplugClientMessageV1::SingleMotorVibrateCmd(m)
      }
      ButtplugClientMessageV0::VorzeA10CycloneCmd(m) => {
        ButtplugClientMessageV1::VorzeA10CycloneCmd(m)
      }
    }
  }
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
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
pub enum ButtplugServerMessageV1 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV1),
  DeviceAdded(DeviceAddedV1),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
}

impl From<ButtplugServerMessageV1> for ButtplugServerMessageV0 {
  fn from(value: ButtplugServerMessageV1) -> Self {
    match value {
      ButtplugServerMessageV1::Ok(m) => ButtplugServerMessageV0::Ok(m),
      ButtplugServerMessageV1::Error(m) => ButtplugServerMessageV0::Error(m),
      ButtplugServerMessageV1::ServerInfo(m) => ButtplugServerMessageV0::ServerInfo(m),
      ButtplugServerMessageV1::DeviceRemoved(m) => ButtplugServerMessageV0::DeviceRemoved(m),
      ButtplugServerMessageV1::ScanningFinished(m) => ButtplugServerMessageV0::ScanningFinished(m),
      ButtplugServerMessageV1::DeviceAdded(m) => ButtplugServerMessageV0::DeviceAdded(m.into()),
      ButtplugServerMessageV1::DeviceList(m) => ButtplugServerMessageV0::DeviceList(m.into()),
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ButtplugDeviceMessageNameV1 {
  VibrateCmd,
  LinearCmd,
  RotateCmd,
  StopDeviceCmd,
  // Deprecated generic commands
  SingleMotorVibrateCmd,
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd,
  LovenseCmd,
  KiirooCmd,
  VorzeA10CycloneCmd,
}

impl PartialOrd for ButtplugDeviceMessageNameV1 {
  fn partial_cmp(&self, other: &ButtplugDeviceMessageNameV1) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for ButtplugDeviceMessageNameV1 {
  fn cmp(&self, other: &ButtplugDeviceMessageNameV1) -> Ordering {
    self.to_string().cmp(&other.to_string())
  }
}
