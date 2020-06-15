// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representations of low level [Buttplug
//! Protocol](https://buttplug-spec.docs.buttplug.io) messages
//!
//! The messages module contains the core communication types for the Buttplug
//! protocol. There are structs for each message type, sometimes with multiple
//! versions of the same message relating to different spec versions. There are
//! also enum types that are used to classify messages into categories, for
//! instance, messages that only should be sent by a client or server.

mod device_added;
mod device_list;
mod device_message_info;
mod device_removed;
mod error;
mod fleshlight_launch_fw12_cmd;
mod kiiroo_cmd;
mod linear_cmd;
mod log;
mod log_level;
mod lovense_cmd;
mod message_attributes;
mod ok;
mod ping;
mod raw_read_cmd;
mod raw_reading;
mod raw_write_cmd;
mod request_device_list;
mod request_log;
mod request_server_info;
mod rotate_cmd;
mod scanning_finished;
pub mod serializer;
mod server_info;
mod single_motor_vibrate_cmd;
mod start_scanning;
mod stop_all_devices;
mod stop_device_cmd;
mod stop_scanning;
mod raw_subscribe_cmd;
mod test;
mod raw_unsubscribe_cmd;
mod vibrate_cmd;
mod vorze_a10_cyclone_cmd;

pub use self::log::Log;
pub use device_added::{DeviceAdded, DeviceAddedV0, DeviceAddedV1};
pub use device_list::{DeviceList, DeviceListV0, DeviceListV1};
pub use device_message_info::{DeviceMessageInfo, MessageAttributesMap};
pub use device_removed::DeviceRemoved;
pub use error::{Error, ErrorCode};
pub use fleshlight_launch_fw12_cmd::FleshlightLaunchFW12Cmd;
pub use kiiroo_cmd::KiirooCmd;
pub use linear_cmd::{LinearCmd, VectorSubcommand};
pub use log_level::LogLevel;
pub use lovense_cmd::LovenseCmd;
pub use message_attributes::MessageAttributes;
pub use ok::Ok;
pub use ping::Ping;
pub use raw_read_cmd::RawReadCmd;
pub use raw_reading::RawReading;
pub use raw_write_cmd::RawWriteCmd;
pub use request_device_list::RequestDeviceList;
pub use request_log::RequestLog;
pub use request_server_info::RequestServerInfo;
pub use rotate_cmd::{RotateCmd, RotationSubcommand};
pub use scanning_finished::ScanningFinished;
pub use server_info::{ServerInfo, ServerInfoV0};
pub use single_motor_vibrate_cmd::SingleMotorVibrateCmd;
pub use start_scanning::StartScanning;
pub use stop_all_devices::StopAllDevices;
pub use stop_device_cmd::StopDeviceCmd;
pub use stop_scanning::StopScanning;
pub use raw_subscribe_cmd::RawSubscribeCmd;
pub use test::Test;
pub use raw_unsubscribe_cmd::RawUnsubscribeCmd;
pub use vibrate_cmd::{VibrateCmd, VibrateSubcommand};
pub use vorze_a10_cyclone_cmd::VorzeA10CycloneCmd;

use crate::core::errors::ButtplugMessageError;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize_json")]
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryFrom;

/// Enum of possible [Buttplug Message
/// Spec](https://buttplug-spec.docs.buttplug.io) versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
#[repr(u32)]
#[cfg_attr(feature = "serialize_json", derive(Serialize_repr, Deserialize_repr))]
pub enum ButtplugMessageSpecVersion {
  Version0 = 0,
  Version1 = 1,
  Version2 = 2,
}

/// Message Id for events sent from the server, which are not in response to a
/// client request.
pub const BUTTPLUG_SERVER_EVENT_ID: u32 = 0;

/// The current latest version of the spec implemented by the library.
pub const BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION: ButtplugMessageSpecVersion =
  ButtplugMessageSpecVersion::Version2;

/// Base trait for all Buttplug Protocol Message Structs. Handles management of
/// message ids, as well as implementing conveinence functions for converting
/// between message structs and various message enums, serialization, etc...
pub trait ButtplugMessage: Send + Sync + Clone {
  /// Returns the id number of the message
  fn get_id(&self) -> u32;
  /// Sets the id number of the message.
  fn set_id(&mut self, id: u32);
  /// True if the message is an event (message id of 0) from the server.
  fn is_server_event(&self) -> bool {
    self.get_id() == BUTTPLUG_SERVER_EVENT_ID
  }
}

pub trait ButtplugClientMessageType: ButtplugMessage {}
pub trait ButtplugServerMessageType: ButtplugMessage {}

/// Adds device index handling to the [ButtplugMessage] trait.
pub trait ButtplugDeviceMessage: ButtplugMessage {
  fn get_device_index(&self) -> u32;
  fn set_device_index(&mut self, id: u32);
}

/// Used in [MessageAttributes][crate::core::messages::MessageAttributes] for denoting message
/// capabilties.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugDeviceMessageType {
  // Generic commands
  VibrateCmd,
  LinearCmd,
  RotateCmd,
  RawWriteCmd,
  RawReadCmd,
  StopDeviceCmd,
  RawSubscribeCmd,
  RawUnsubscribeCmd,
  // Deprecated generic commands
  SingleMotorVibrateCmd,
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd,
  LovenseCmd,
  KiirooCmd,
  VorzeA10CycloneCmd,
  BatteryLevelCmd,
  RSSILevelCmd,
  // To Add:
  // PatternCmd
  // BatteryLevelReading
  // RSSILevelReading
  // ShockCmd?
  // ToneEmitterCmd?
}

/// Represents all possible messages a
/// [ButtplugClient][crate::client::ButtplugClient] can send to a
/// [ButtplugServer][crate::server::ButtplugServer].
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugClientMessageType, FromSpecificButtplugMessage,
)]
pub enum ButtplugClientMessage {
  Ping(Ping),
  RequestLog(RequestLog),
  // Handshake messages
  RequestServerInfo(RequestServerInfo),
  // Device enumeration messages
  StartScanning(StartScanning),
  StopScanning(StopScanning),
  RequestDeviceList(RequestDeviceList),
  // Generic commands
  StopAllDevices(StopAllDevices),
  VibrateCmd(VibrateCmd),
  LinearCmd(LinearCmd),
  RotateCmd(RotateCmd),
  RawWriteCmd(RawWriteCmd),
  RawReadCmd(RawReadCmd),
  StopDeviceCmd(StopDeviceCmd),
  RawSubscribeCmd(RawSubscribeCmd),
  RawUnsubscribeCmd(RawUnsubscribeCmd),
  // Deprecated generic commands
  SingleMotorVibrateCmd(SingleMotorVibrateCmd),
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
  LovenseCmd(LovenseCmd),
  KiirooCmd(KiirooCmd),
  VorzeA10CycloneCmd(VorzeA10CycloneCmd),
  // To Add:
  // PatternCmd
  // BatteryLevelCmd
  // RSSILevelCmd
  // ShockCmd?
  // ToneEmitterCmd?
}

/// Represents all possible messages a
/// [ButtplugServer][crate::server::ButtplugServer] can send to a
/// [ButtplugClient][crate::client::ButtplugClient].
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugServerMessageType, FromSpecificButtplugMessage,
)]
pub enum ButtplugServerMessage {
  // Status messages
  Ok(Ok),
  Error(Error),
  Test(Test),
  Log(Log),
  // Handshake messages
  ServerInfo(ServerInfo),
  // Device enumeration messages
  DeviceList(DeviceList),
  DeviceAdded(DeviceAdded),
  DeviceRemoved(DeviceRemoved),
  ScanningFinished(ScanningFinished),
  // Generic commands
  RawReading(RawReading),
  // To Add:
  // BatteryLevelReading
  // RSSILevelReading
}

/// Type alias for the latest version of client-to-server messages.
pub type ButtplugCurrentSpecClientMessage = ButtplugSpecV2ClientMessage;
/// Type alias for the latest version of server-to-client messages.
pub type ButtplugCurrentSpecServerMessage = ButtplugSpecV2ServerMessage;

/// Represents all client-to-server messages in v2 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugClientMessageType,
  FromSpecificButtplugMessage,
  TryFromButtplugClientMessage,
)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugSpecV2ClientMessage {
  RequestLog(RequestLog),
  // Handshake messages
  RequestServerInfo(RequestServerInfo),
  // Device enumeration messages
  StartScanning(StartScanning),
  StopScanning(StopScanning),
  RequestDeviceList(RequestDeviceList),
  // Generic commands
  StopAllDevices(StopAllDevices),
  VibrateCmd(VibrateCmd),
  LinearCmd(LinearCmd),
  RotateCmd(RotateCmd),
  RawWriteCmd(RawWriteCmd),
  RawReadCmd(RawReadCmd),
  StopDeviceCmd(StopDeviceCmd),
  RawSubscribeCmd(RawSubscribeCmd),
  RawUnsubscribeCmd(RawUnsubscribeCmd),
  // To Add:
  // PatternCmd
  // BatteryLevelCmd
  // RSSILevelCmd
  // ShockCmd?
  // ToneEmitterCmd?
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugServerMessageType,
  FromSpecificButtplugMessage,
  TryFromButtplugServerMessage,
)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugSpecV2ServerMessage {
  // Status messages
  Ok(Ok),
  Error(Error),
  Log(Log),
  // Handshake messages
  ServerInfo(ServerInfo),
  // Device enumeration messages
  DeviceList(DeviceList),
  DeviceAdded(DeviceAdded),
  DeviceRemoved(DeviceRemoved),
  ScanningFinished(ScanningFinished),
  // Generic commands
  RawReading(RawReading),
  // To Add:
  // BatteryLevelReading
  // RSSILevelReading
}

/// Represents all client-to-server messages in v1 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugClientMessageType, TryFromButtplugClientMessage,
)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV1ClientMessage {
  RequestLog(RequestLog),
  // Handshake messages
  RequestServerInfo(RequestServerInfo),
  // Device enumeration messages
  StartScanning(StartScanning),
  StopScanning(StopScanning),
  RequestDeviceList(RequestDeviceList),
  // Generic commands
  StopAllDevices(StopAllDevices),
  VibrateCmd(VibrateCmd),
  LinearCmd(LinearCmd),
  RotateCmd(RotateCmd),
  StopDeviceCmd(StopDeviceCmd),
  // Deprecated generic commands
  SingleMotorVibrateCmd(SingleMotorVibrateCmd),
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
  LovenseCmd(LovenseCmd),
  KiirooCmd(KiirooCmd),
  VorzeA10CycloneCmd(VorzeA10CycloneCmd),
}

/// Represents all server-to-client messages in v2 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, ButtplugMessage, ButtplugServerMessageType)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV1ServerMessage {
  // Status messages
  Ok(Ok),
  Error(Error),
  Log(Log),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV1),
  DeviceAdded(DeviceAddedV1),
  DeviceRemoved(DeviceRemoved),
  ScanningFinished(ScanningFinished),
}

// TODO This was implementated as a derive, but for some reason the .into()
// calls wouldn't work correctly when used as a device. If the actual
// implementation is here, things work fine. Luckily it won't ever be changed
// much.
impl TryFrom<ButtplugServerMessage> for ButtplugSpecV1ServerMessage {
  type Error = ButtplugMessageError;
  fn try_from(msg: ButtplugServerMessage) -> Result<Self, ButtplugMessageError> {
    match msg {
      ButtplugServerMessage::Ok(msg) => Ok(ButtplugSpecV1ServerMessage::Ok(msg)),
      ButtplugServerMessage::Error(msg) => Ok(ButtplugSpecV1ServerMessage::Error(msg)),
      ButtplugServerMessage::Log(msg) => Ok(ButtplugSpecV1ServerMessage::Log(msg)),
      ButtplugServerMessage::ServerInfo(msg) => {
        Ok(ButtplugSpecV1ServerMessage::ServerInfo(msg.into()))
      }
      ButtplugServerMessage::DeviceList(msg) => {
        Ok(ButtplugSpecV1ServerMessage::DeviceList(msg.into()))
      }
      ButtplugServerMessage::DeviceAdded(msg) => {
        Ok(ButtplugSpecV1ServerMessage::DeviceAdded(msg.into()))
      }
      ButtplugServerMessage::DeviceRemoved(msg) => {
        Ok(ButtplugSpecV1ServerMessage::DeviceRemoved(msg))
      }
      ButtplugServerMessage::ScanningFinished(msg) => {
        Ok(ButtplugSpecV1ServerMessage::ScanningFinished(msg))
      }
      _ => Err(ButtplugMessageError::new(
        "ButtplugServerMessage cannot be converted to #name",
      )),
    }
  }
}

/// Represents all client-to-server messages in v0 of the Buttplug Spec
#[derive(
  Debug, Clone, PartialEq, ButtplugMessage, ButtplugClientMessageType, TryFromButtplugClientMessage,
)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV0ClientMessage {
  RequestLog(RequestLog),
  // Handshake messages
  RequestServerInfo(RequestServerInfo),
  // Device enumeration messages
  StartScanning(StartScanning),
  StopScanning(StopScanning),
  RequestDeviceList(RequestDeviceList),
  // Generic commands
  StopAllDevices(StopAllDevices),
  StopDeviceCmd(StopDeviceCmd),
  // Deprecated generic commands
  SingleMotorVibrateCmd(SingleMotorVibrateCmd),
  // Deprecated device specific commands
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
  LovenseCmd(LovenseCmd),
  KiirooCmd(KiirooCmd),
  VorzeA10CycloneCmd(VorzeA10CycloneCmd),
}

/// Represents all server-to-client messages in v0 of the Buttplug Spec
#[derive(Debug, Clone, PartialEq, ButtplugMessage, ButtplugServerMessageType)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV0ServerMessage {
  // Status messages
  Ok(Ok),
  Error(Error),
  Log(Log),
  // Handshake messages
  ServerInfo(ServerInfoV0),
  // Device enumeration messages
  DeviceList(DeviceListV0),
  DeviceAdded(DeviceAddedV0),
  DeviceRemoved(DeviceRemoved),
  ScanningFinished(ScanningFinished),
}

// TODO This was implementated as a derive, but for some reason the .into()
// calls wouldn't work correctly when used as a device. If the actual
// implementation is here, things work fine. Luckily it won't ever be changed
// much.
impl TryFrom<ButtplugServerMessage> for ButtplugSpecV0ServerMessage {
  type Error = ButtplugMessageError;
  fn try_from(msg: ButtplugServerMessage) -> Result<Self, ButtplugMessageError> {
    match msg {
      ButtplugServerMessage::Ok(msg) => Ok(ButtplugSpecV0ServerMessage::Ok(msg)),
      ButtplugServerMessage::Error(msg) => Ok(ButtplugSpecV0ServerMessage::Error(msg)),
      ButtplugServerMessage::Log(msg) => Ok(ButtplugSpecV0ServerMessage::Log(msg)),
      ButtplugServerMessage::ServerInfo(msg) => {
        Ok(ButtplugSpecV0ServerMessage::ServerInfo(msg.into()))
      }
      ButtplugServerMessage::DeviceList(msg) => {
        Ok(ButtplugSpecV0ServerMessage::DeviceList(msg.into()))
      }
      ButtplugServerMessage::DeviceAdded(msg) => {
        Ok(ButtplugSpecV0ServerMessage::DeviceAdded(msg.into()))
      }
      ButtplugServerMessage::DeviceRemoved(msg) => {
        Ok(ButtplugSpecV0ServerMessage::DeviceRemoved(msg))
      }
      ButtplugServerMessage::ScanningFinished(msg) => {
        Ok(ButtplugSpecV0ServerMessage::ScanningFinished(msg))
      }
      _ => Err(ButtplugMessageError::new(
        "ButtplugServerMessage cannot be converted to ButtplugSepcV0OutMessage",
      )),
    }
  }
}
/// Represents messages that should go to the
/// [DeviceManager][crate::server::device_manager::DeviceManager] of a
/// [ButtplugServer](crate::server::ButtplugServer)
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugMessage,
  ButtplugClientMessageType,
  FromSpecificButtplugMessage,
  TryFromButtplugClientMessage,
)]
pub enum ButtplugDeviceManagerMessageUnion {
  RequestDeviceList(RequestDeviceList),
  StopAllDevices(StopAllDevices),
  StartScanning(StartScanning),
  StopScanning(StopScanning),
}

/// Represents all possible device command message types.
#[derive(
  Debug,
  Clone,
  PartialEq,
  ButtplugDeviceMessage,
  ButtplugClientMessageType,
  FromSpecificButtplugMessage,
  TryFromButtplugClientMessage,
)]
pub enum ButtplugDeviceCommandMessageUnion {
  FleshlightLaunchFW12Cmd(FleshlightLaunchFW12Cmd),
  SingleMotorVibrateCmd(SingleMotorVibrateCmd),
  VorzeA10CycloneCmd(VorzeA10CycloneCmd),
  KiirooCmd(KiirooCmd),
  // No LovenseCmd, it was never implemented anywhere.
  VibrateCmd(VibrateCmd),
  LinearCmd(LinearCmd),
  RotateCmd(RotateCmd),
  RawWriteCmd(RawWriteCmd),
  RawReadCmd(RawReadCmd),
  StopDeviceCmd(StopDeviceCmd),
  RawSubscribeCmd(RawSubscribeCmd),
  RawUnsubscribeCmd(RawUnsubscribeCmd),
}
