// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugMessageFinalizer,
  ErrorV0,
  OkV0,
  OutputCmdV4,
  PingV0,
  RequestDeviceListV0,
  RequestServerInfoV4,
  ScanningFinishedV0,
  ServerInfoV4,
  StartScanningV0,
  StopAllDevicesV4,
  StopDeviceCmdV4,
  StopScanningV0,
  v4::input_cmd::InputCmdV4,
};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use super::{DeviceListV4, InputReadingV4};

/// Represents all client-to-server messages in v4 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[enum_dispatch(ButtplugMessage, ButtplugMessageValidator)]
pub enum ButtplugClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV4),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV4),
  StopAllDevices(StopAllDevicesV4),
  OutputCmd(OutputCmdV4),
  InputCmd(InputCmdV4),
}

impl ButtplugMessageFinalizer for ButtplugClientMessageV4 {
}

/// Represents all server-to-client messages in v4 of the Buttplug Spec
#[derive(Debug, Clone, Serialize, Deserialize)]
#[enum_dispatch(ButtplugMessage, ButtplugMessageValidator)]
pub enum ButtplugServerMessageV4 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  // Handshake messages
  ServerInfo(ServerInfoV4),
  // Device enumeration messages
  DeviceList(DeviceListV4),
  ScanningFinished(ScanningFinishedV0),
  // Sensor commands
  InputReading(InputReadingV4),
}

impl ButtplugMessageFinalizer for ButtplugServerMessageV4 {
  fn finalize(&mut self) {
    if let ButtplugServerMessageV4::DeviceList(dl) = self {
      dl.finalize()
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ButtplugDeviceMessageNameV4 {
  StopDeviceCmd,
  InputCmd,
  OutputCmd,
}
