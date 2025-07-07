// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v4::input_cmd::InputCmdV4,
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  ErrorV0,
  OkV0,
  OutputCmdV4,
  PingV0,
  RequestDeviceListV0,
  RequestServerInfoV4,
  ScanningFinishedV0,
  ServerInfoV4,
  StartScanningV0,
  StopAllDevicesV0,
  StopDeviceCmdV0,
  StopScanningV0,
};
use serde::{Deserialize, Serialize};

use super::{DeviceListV4, InputReadingV4};

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
pub enum ButtplugClientMessageV4 {
  // Handshake messages
  RequestServerInfo(RequestServerInfoV4),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  OutputCmd(OutputCmdV4),
  InputCmd(InputCmdV4),
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
    match self {
      ButtplugServerMessageV4::DeviceList(dl) => dl.finalize(),
      _ => (),
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ButtplugDeviceMessageNameV4 {
  StopDeviceCmd,
  InputCmd,
  OutputCmd,
}
