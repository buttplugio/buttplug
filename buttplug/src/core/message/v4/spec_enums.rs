// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  v4::sensor_cmd::SensorCmdV4, ActuatorCmdV4, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceRemovedV0, ErrorV0, OkV0, PingV0, RawCmdV4, RawReadingV2, RequestDeviceListV0, RequestServerInfoV4, ScanningFinishedV0, ServerInfoV4, StartScanningV0, StopAllDevicesV0, StopDeviceCmdV0, StopScanningV0
};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{
  DeviceAddedV4,
  DeviceListV4,
  SensorReadingV4,
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
  RequestServerInfo(RequestServerInfoV4),
  Ping(PingV0),
  // Device enumeration messages
  StartScanning(StartScanningV0),
  StopScanning(StopScanningV0),
  RequestDeviceList(RequestDeviceListV0),
  // Generic commands
  StopDeviceCmd(StopDeviceCmdV0),
  StopAllDevices(StopAllDevicesV0),
  ActuatorCmd(ActuatorCmdV4),
  SensorCmd(SensorCmdV4),
  RawCmd(RawCmdV4),
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
  ServerInfo(ServerInfoV4),
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
