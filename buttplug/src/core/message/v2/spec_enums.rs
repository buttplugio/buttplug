// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceRemovedV0, ErrorV0, LinearCmdV1, OkV0, PingV0, RequestDeviceListV0, RequestServerInfoV1, RotateCmdV1, ScanningFinishedV0, StartScanningV0, StopAllDevicesV0, StopDeviceCmdV0, StopScanningV0, VibrateCmdV1
};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{BatteryLevelCmdV2, BatteryLevelReadingV2, DeviceAddedV2, DeviceListV2, RSSILevelCmdV2, RSSILevelReadingV2, RawReadCmdV2, RawReadingV2, RawSubscribeCmdV2, RawUnsubscribeCmdV2, RawWriteCmdV2, ServerInfoV2};



/// Represents all client-to-server messages in v2 of the Buttplug Spec
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
  RawWriteCmd(RawWriteCmdV2),
  RawReadCmd(RawReadCmdV2),
  StopDeviceCmd(StopDeviceCmdV0),
  RawSubscribeCmd(RawSubscribeCmdV2),
  RawUnsubscribeCmd(RawUnsubscribeCmdV2),
  // Sensor commands
  BatteryLevelCmd(BatteryLevelCmdV2),
  RSSILevelCmd(RSSILevelCmdV2),
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
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
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
  // Generic commands
  RawReading(RawReadingV2),
  // Sensor commands
  BatteryLevelReading(BatteryLevelReadingV2),
  RSSILevelReading(RSSILevelReadingV2),
}

