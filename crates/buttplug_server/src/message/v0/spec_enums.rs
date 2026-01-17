use std::cmp::Ordering;

use super::*;
use crate::message::{RequestServerInfoV1, v0::stop_device_cmd::StopDeviceCmdV0};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator, PingV0},
};

use serde::{Deserialize, Serialize};

/// Represents all client-to-server messages in v0 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, ButtplugMessage, derive_more::From, Serialize, Deserialize)]
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

impl ButtplugMessageFinalizer for ButtplugClientMessageV0 {}

impl ButtplugMessageValidator for ButtplugClientMessageV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugClientMessageV0::Ping(msg) => msg.is_valid(),
      ButtplugClientMessageV0::RequestServerInfo(msg) => msg.is_valid(),
      ButtplugClientMessageV0::StartScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV0::StopScanning(msg) => msg.is_valid(),
      ButtplugClientMessageV0::RequestDeviceList(msg) => msg.is_valid(),
      ButtplugClientMessageV0::StopAllDevices(msg) => msg.is_valid(),
      ButtplugClientMessageV0::StopDeviceCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV0::SingleMotorVibrateCmd(msg) => msg.is_valid(),
      ButtplugClientMessageV0::FleshlightLaunchFW12Cmd(msg) => msg.is_valid(),
      ButtplugClientMessageV0::VorzeA10CycloneCmd(msg) => msg.is_valid(),
    }
  }
}

/// Represents all server-to-client messages in v0 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, ButtplugMessage, derive_more::From, Serialize, Deserialize)]
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

impl ButtplugMessageFinalizer for ButtplugServerMessageV0 {}

impl ButtplugMessageValidator for ButtplugServerMessageV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    match self {
      ButtplugServerMessageV0::Ok(msg) => msg.is_valid(),
      ButtplugServerMessageV0::Error(msg) => msg.is_valid(),
      ButtplugServerMessageV0::ServerInfo(msg) => msg.is_valid(),
      ButtplugServerMessageV0::DeviceList(msg) => msg.is_valid(),
      ButtplugServerMessageV0::DeviceAdded(msg) => msg.is_valid(),
      ButtplugServerMessageV0::DeviceRemoved(msg) => msg.is_valid(),
      ButtplugServerMessageV0::ScanningFinished(msg) => msg.is_valid(),
    }
  }
}

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
