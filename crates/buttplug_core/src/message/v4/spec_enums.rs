// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
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
  StopAllDevicesV4,
  StopDeviceCmdV4,
  StopScanningV0,
  v4::input_cmd::InputCmdV4,
};
use serde::{Deserialize, Serialize};

use super::{DeviceListV4, InputReadingV4};

/// Represents all client-to-server messages in v4 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, ButtplugMessage, derive_more::From, Serialize, Deserialize)]
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

impl ButtplugMessageValidator for ButtplugClientMessageV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugClientMessageV4::RequestServerInfo(msg) => msg.is_valid(),
      ButtplugClientMessageV4::Ping(msg) => msg.is_valid(),
      ButtplugClientMessageV4::StartScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV4::StopScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV4::RequestDeviceList(msg) => msg.is_valid(),
      ButtplugClientMessageV4::StopDeviceCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV4::StopAllDevices(msg) => msg.is_valid(),
      ButtplugClientMessageV4::OutputCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV4::InputCmd(msg) => msg.is_valid(),
    }
  }
}

/// Represents all server-to-client messages in v4 of the Buttplug Spec
#[derive(Debug, Clone, ButtplugMessage, derive_more::From, Serialize, Deserialize)]
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

impl ButtplugMessageValidator for ButtplugServerMessageV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugServerMessageV4::Ok(msg) => msg.is_valid(),
      ButtplugServerMessageV4::Error(msg) => msg.is_valid(),
      ButtplugServerMessageV4::ServerInfo(msg) => msg.is_valid(),
      ButtplugServerMessageV4::DeviceList(msg) => msg.is_valid(),
      ButtplugServerMessageV4::ScanningFinished(msg) => msg.is_valid(),
      ButtplugServerMessageV4::InputReading(msg) => msg.is_valid(),
    }
  }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash, Display)]
pub enum ButtplugDeviceMessageNameV4 {
  StopDeviceCmd,
  InputCmd,
  OutputCmd,
}
