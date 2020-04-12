// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Structs representing low level [Buttplug
//! Protocol](https://buttplug-spec.docs.buttplug.io) messages

mod device_added;
mod device_removed;
mod device_list;
mod device_message_info;
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
mod server_info;
mod single_motor_vibrate_cmd;
mod start_scanning;
mod stop_scanning;
mod stop_all_devices;
mod stop_device_cmd;
mod subscribe_cmd;
mod test;
mod unsubscribe_cmd;
mod vibrate_cmd;
mod vorze_a10_cyclone_cmd;

pub use device_added::{DeviceAdded, DeviceAddedV1, DeviceAddedV0};
pub use device_removed::DeviceRemoved;
pub use device_list::{DeviceList, DeviceListV1, DeviceListV0};
pub use device_message_info::{DeviceMessageInfo, MessageAttributesMap};
pub use error::{Error, ErrorCode};
pub use fleshlight_launch_fw12_cmd::FleshlightLaunchFW12Cmd;
pub use kiiroo_cmd::KiirooCmd;
pub use linear_cmd::{LinearCmd, VectorSubcommand};
pub use self::log::Log;
pub use log_level::LogLevel;
pub use lovense_cmd::LovenseCmd;
pub use message_attributes::MessageAttributes;
pub use ok::Ok;
pub use ping::Ping;
pub use raw_read_cmd::RawReadCmd;
pub use raw_write_cmd::RawWriteCmd;
pub use raw_reading::RawReading;
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
pub use subscribe_cmd::SubscribeCmd;
pub use test::Test;
pub use unsubscribe_cmd::UnsubscribeCmd;
pub use vibrate_cmd::{VibrateCmd, VibrateSubcommand};
pub use vorze_a10_cyclone_cmd::VorzeA10CycloneCmd;

use crate::{
    core::errors::ButtplugMessageError,
    util::json::JSONValidator
};
use std::convert::TryFrom;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize_json")]
use serde_repr::{Deserialize_repr, Serialize_repr};

static MESSAGE_JSON_SCHEMA: &str =
    include_str!("../../../dependencies/buttplug-schema/schema/buttplug-schema.json");

pub fn create_message_validator() -> JSONValidator {
    JSONValidator::new(MESSAGE_JSON_SCHEMA)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
#[repr(u32)]
#[cfg_attr(feature = "serialize_json", derive(Serialize_repr, Deserialize_repr))]
pub enum ButtplugMessageSpecVersion {
    Version0 = 0,
    Version1 = 1,
    Version2 = 2,
}

pub const BUTTPLUG_CURRENT_MESSAGE_SPEC_VERSION: ButtplugMessageSpecVersion = ButtplugMessageSpecVersion::Version2;

/// Base trait for all Buttplug Protocol Message Structs. Handles management of
/// message ids, as well as implementing conveinence functions for converting
/// between message structs and [ButtplugMessageUnion] enums, serialization, etc...
pub trait ButtplugMessage: Send + Sync + Clone {
    /// Returns the id number of the message
    fn get_id(&self) -> u32;
    /// Sets the id number of the message.
    fn set_id(&mut self, id: u32);
    /// Returns the message as a string in Buttplug JSON Protocol format.
    #[cfg(feature = "serialize_json")]
    fn as_protocol_json(self) -> String
    where
        Self: ButtplugMessage + Serialize + Deserialize<'static>,
    {
        serde_json::to_string(&[&self]).unwrap()
    }
}

pub trait ButtplugDeviceMessage: ButtplugMessage {
    fn get_device_index(&self) -> u32;
    fn set_device_index(&mut self, id: u32);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugMessageType {
    // Status messages
    Ok,
    Error,
    Ping,
    Test,
    RequestLog,
    Log,
    // Handshake messages
    RequestServerInfo,
    ServerInfo,
    // Device enumeration messages
    DeviceList,
    DeviceAdded,
    DeviceRemoved,
    StartScanning,
    StopScanning,
    ScanningFinished,
    RequestDeviceList,
    // Generic commands
    StopAllDevices,
    VibrateCmd,
    LinearCmd,
    RotateCmd,
    RawWriteCmd,
    RawReadCmd,
    StopDeviceCmd,
    RawReading,
    SubscribeCmd,
    UnsubscribeCmd,
    BatteryLevelCmd,
    RSSILevelCmd,
    // Deprecated generic commands
    SingleMotorVibrateCmd,
    // Deprecated device specific commands
    FleshlightLaunchFW12Cmd,
    LovenseCmd,
    KiirooCmd,
    VorzeA10CycloneCmd,
    // To Add:
    // PatternCmd
    // BatteryLevelReading
    // RSSILevelReading
    // ShockCmd?
    // ToneEmitterCmd?
}

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
    SubscribeCmd,
    UnsubscribeCmd,
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

#[derive(Debug, Clone, PartialEq, ButtplugMessage, FromSpecificButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))] // TODO Remove this after testing!
pub enum ButtplugInMessage {
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
    SubscribeCmd(SubscribeCmd),
    UnsubscribeCmd(UnsubscribeCmd),
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

#[derive(Debug, Clone, PartialEq, ButtplugMessage, FromSpecificButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))] // TODO Remove this after testing!
pub enum ButtplugOutMessage {
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

pub type ButtplugClientInMessage = ButtplugSpecV2InMessage;
pub type ButtplugClientOutMessage = ButtplugSpecV2OutMessage;

#[derive(Debug, Clone, PartialEq, ButtplugMessage, FromSpecificButtplugMessage, TryFromButtplugInMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugSpecV2InMessage {
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
    SubscribeCmd(SubscribeCmd),
    UnsubscribeCmd(UnsubscribeCmd),
    // To Add:
    // PatternCmd
    // BatteryLevelCmd
    // RSSILevelCmd
    // ShockCmd?
    // ToneEmitterCmd?
}

#[derive(Debug, Clone, PartialEq, ButtplugMessage, FromSpecificButtplugMessage, TryFromButtplugOutMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum ButtplugSpecV2OutMessage {
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

#[derive(Debug, Clone, PartialEq, ButtplugMessage, TryFromButtplugInMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV1InMessage {
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

#[derive(Debug, Clone, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV1OutMessage {
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
impl TryFrom<ButtplugOutMessage> for ButtplugSpecV1OutMessage {
    type Error = ButtplugMessageError;
    fn try_from(msg: ButtplugOutMessage)
     -> Result<Self, ButtplugMessageError> {
        match msg {
            ButtplugOutMessage::Ok(msg) =>
            Ok(ButtplugSpecV1OutMessage::Ok(msg.into())),
            ButtplugOutMessage::Error(msg) =>
            Ok(ButtplugSpecV1OutMessage::Error(msg.into())),
            ButtplugOutMessage::Log(msg) =>
            Ok(ButtplugSpecV1OutMessage::Log(msg.into())),
            ButtplugOutMessage::ServerInfo(msg) =>
            Ok(ButtplugSpecV1OutMessage::ServerInfo(msg.into())),
            ButtplugOutMessage::DeviceList(msg) =>
            Ok(ButtplugSpecV1OutMessage::DeviceList(msg.into())),
            ButtplugOutMessage::DeviceAdded(msg) =>
            Ok(ButtplugSpecV1OutMessage::DeviceAdded(msg.into())),
            ButtplugOutMessage::DeviceRemoved(msg) =>
            Ok(ButtplugSpecV1OutMessage::DeviceRemoved(msg.into())),
            ButtplugOutMessage::ScanningFinished(msg) =>
            Ok(ButtplugSpecV1OutMessage::ScanningFinished(msg.into())),
            _ =>
            Err(ButtplugMessageError::new("ButtplugOutMessage cannot be converted to #name")),
        }
    }
}


#[derive(Debug, Clone, PartialEq, ButtplugMessage, TryFromButtplugInMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV0InMessage {
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

#[derive(Debug, Clone, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub(crate) enum ButtplugSpecV0OutMessage {
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
impl TryFrom<ButtplugOutMessage> for ButtplugSpecV0OutMessage {
    type Error = ButtplugMessageError;
    fn try_from(msg: ButtplugOutMessage)
     -> Result<Self, ButtplugMessageError> {
        match msg {
            ButtplugOutMessage::Ok(msg) =>
            Ok(ButtplugSpecV0OutMessage::Ok(msg.into())),
            ButtplugOutMessage::Error(msg) =>
            Ok(ButtplugSpecV0OutMessage::Error(msg.into())),
            ButtplugOutMessage::Log(msg) =>
            Ok(ButtplugSpecV0OutMessage::Log(msg.into())),
            ButtplugOutMessage::ServerInfo(msg) =>
            Ok(ButtplugSpecV0OutMessage::ServerInfo(msg.into())),
            ButtplugOutMessage::DeviceList(msg) =>
            Ok(ButtplugSpecV0OutMessage::DeviceList(msg.into())),
            ButtplugOutMessage::DeviceAdded(msg) =>
            Ok(ButtplugSpecV0OutMessage::DeviceAdded(msg.into())),
            ButtplugOutMessage::DeviceRemoved(msg) =>
            Ok(ButtplugSpecV0OutMessage::DeviceRemoved(msg.into())),
            ButtplugOutMessage::ScanningFinished(msg) =>
            Ok(ButtplugSpecV0OutMessage::ScanningFinished(msg.into())),
            _ =>
            Err(ButtplugMessageError::new("ButtplugOutMessage cannot be converted to ButtplugSepcV0OutMessage")),
        }
    }
}
/// Messages that should never be received from the client.
#[derive(
    Debug, 
    Clone, 
    PartialEq, 
    ButtplugMessage,
    FromSpecificButtplugMessage,
    TryFromButtplugInMessage,
)]
pub enum ButtplugDeviceManagerMessageUnion {
    RequestDeviceList(RequestDeviceList),
    StopAllDevices(StopAllDevices),
    StartScanning(StartScanning),
    StopScanning(StopScanning),
}

/// Messages that should be routed to device instances.
#[derive(
    Debug,
    Clone,
    PartialEq,
    ButtplugDeviceMessage,
    FromSpecificButtplugMessage,
    TryFromButtplugInMessage,
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
    SubscribeCmd(SubscribeCmd),
    UnsubscribeCmd(UnsubscribeCmd),
}
