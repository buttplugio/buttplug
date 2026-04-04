// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::cmp::Ordering;

use super::*;
use crate::message::{RequestServerInfoV1, v0::stop_device_cmd::StopDeviceCmdV0};
use buttplug_core::message::PingV0;

use serde::{Deserialize, Serialize};

/// Represents all client-to-server messages in v0 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, derive_more::From, Serialize, Deserialize)]
pub enum ButtplugClientMessageV0 {
  Ping(PingV0),
  // Handshake messages
  //
  // We use RequestServerInfoV1 here, as the only difference between v0 and v1 was passing the spec
  // version. If the spec version doesn't exist, we automatically set the spec version to 0.
  RequestServerInfo(RequestServerInfoV1),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopAllDevices(StopAllDevicesV0),
  StopDeviceCmd(StopDeviceCmdV0),
  // Deprecated generic commands
  SingleMotorVibrateCmd(SingleMotorVibrateCmdV0),
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12CmdV0),
  VorzeA10CycloneCmd(VorzeA10CycloneCmdV0),
}

impl_message_enum_traits!(ButtplugClientMessageV0 {
  Ping,
  RequestServerInfo,
  StartScanning,
  StopScanning,
  RequestDeviceList,
  StopAllDevices,
  StopDeviceCmd,
  SingleMotorVibrateCmd,
  FleshlightLaunchFW12Cmd,
  VorzeA10CycloneCmd,
});
impl buttplug_core::message::ButtplugMessageFinalizer for ButtplugClientMessageV0 {}

/// Represents all server-to-client messages in v0 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, derive_more::From, Serialize, Deserialize)]
pub enum ButtplugServerMessageV0 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV0),
  DeviceAdded(DeviceAddedV0),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
}

impl_message_enum_traits!(ButtplugServerMessageV0 {
  Ok,
  Error,
  ServerInfo,
  DeviceList,
  DeviceAdded,
  DeviceRemoved,
  ScanningFinished,
});
impl buttplug_core::message::ButtplugMessageFinalizer for ButtplugServerMessageV0 {}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display, Serialize, Deserialize)]
pub enum ButtplugDeviceMessageNameV0 {
  StopDeviceCmd,
  // Deprecated generic commands
  SingleMotorVibrateCmd,
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd,
  LovenseCmd,
  KiirooCmd,
  VorzeA10CycloneCmd,
}

impl PartialOrd for ButtplugDeviceMessageNameV0 {
  fn partial_cmp(&self, other: &ButtplugDeviceMessageNameV0) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for ButtplugDeviceMessageNameV0 {
  fn cmp(&self, other: &ButtplugDeviceMessageNameV0) -> Ordering {
    self.to_string().cmp(&other.to_string())
  }
}
