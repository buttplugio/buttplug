// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
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
      RequestServerInfoV1,
      ScanningFinishedV0,
      StartScanningV0,
      StopAllDevicesV0,
      StopDeviceCmdV0,
      StopScanningV0,
    },
  },
  server::message::v1::{
    ButtplugClientMessageV1,
    ButtplugServerMessageV1,
    LinearCmdV1,
    RotateCmdV1,
    VibrateCmdV1,
  },
};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{
  BatteryLevelCmdV2,
  BatteryLevelReadingV2,
  DeviceAddedV2,
  DeviceListV2,
  RSSILevelCmdV2,
  RSSILevelReadingV2,
  RawReadCmdV2,
  RawReadingV2,
  RawSubscribeCmdV2,
  RawUnsubscribeCmdV2,
  RawWriteCmdV2,
  ServerInfoV2,
};

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
      ButtplugClientMessageV1::RequestLog(_) => {
        // Log was a huge security hole, as we'd just send our server logs to whomever asked, which
        // contain all sorts of identifying information. Always return an error here.
        Err(ButtplugMessageError::MessageConversionError(
          "RequestLog is no longer allowed by any version of Buttplug.".to_owned(),
        ))
      }
      ButtplugClientMessageV1::KiirooCmd(_) => {
        // No device protocol implementation ever worked with KiirooCmd, so no one ever should've
        // used it. We'll just return an error if we ever see it.
        Err(ButtplugMessageError::MessageConversionError(
          "KiirooCmd is not implemented. Please update the client software to use a newer command"
            .to_owned(),
        ))
      }
      ButtplugClientMessageV1::LovenseCmd(_) => {
        // LovenseCmd allowed users to directly send strings to a Lovense device, which was a Bad
        // Idea. Will always return an error.
        Err(ButtplugMessageError::MessageConversionError(
          "LovenseCmd is not implemented. Please update the client software to use a newer command"
            .to_owned(),
        ))
      }
      _ => Err(ButtplugMessageError::MessageConversionError(format!(
        "Cannot convert message {:?} to current message spec while lacking state.",
        value
      ))),
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
      ButtplugServerMessageV2::RSSILevelReading(_) => {
        ButtplugServerMessageV1::Error(ErrorV0::from(ButtplugError::from(
          ButtplugMessageError::MessageConversionError(
            "RSSILevelReading cannot be converted to Buttplug Message Spec V1".to_owned(),
          ),
        )))
      }
      ButtplugServerMessageV2::RawReading(_) => ButtplugServerMessageV1::Error(ErrorV0::from(
        ButtplugError::from(ButtplugMessageError::MessageConversionError(
          "RawReading cannot be converted to Buttplug Message Spec V1".to_owned(),
        )),
      )),
    }
  }
}
