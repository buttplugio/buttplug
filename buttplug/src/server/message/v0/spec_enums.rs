use std::cmp::Ordering;

use super::*;
use crate::{core::{
  errors::ButtplugMessageError,
  message::{
    ButtplugMessage,
    ButtplugMessageFinalizer,
    ButtplugMessageValidator,
    PingV0,
  },
}, server::message::RequestServerInfoV1};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Represents all client-to-server messages in v0 of the Buttplug Spec
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
pub enum ButtplugClientMessageV0 {
  RequestLog(RequestLogV0),
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
  LovenseCmd(LovenseCmdV0),
  KiirooCmd(KiirooCmdV0),
  VorzeA10CycloneCmd(VorzeA10CycloneCmdV0),
}

/// Represents all server-to-client messages in v0 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugMessageValidator, ButtplugMessageFinalizer,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum ButtplugServerMessageV0 {
  // Status messages
  Ok(OkV0),
  Error(ErrorV0),
  Log(LogV0),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV0),
  DeviceAdded(DeviceAddedV0),
  DeviceRemoved(DeviceRemovedV0),
  ScanningFinished(ScanningFinishedV0),
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