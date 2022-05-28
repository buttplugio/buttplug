// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use serde::{Deserialize, Serialize};
use serde_repr::*;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum OutgoingLovenseData {
  Raw(String),
  Message(LovenseDongleOutgoingMessage),
}

#[derive(Debug)]
pub enum LovenseDeviceCommand {
  DongleFound(
    Sender<OutgoingLovenseData>,
    Receiver<LovenseDongleIncomingMessage>,
  ),
  StartScanning,
  StopScanning,
}

#[repr(u16)]
#[derive(Serialize_repr, Deserialize_repr, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LovenseDongleResultCode {
  DongleInitialized = 100,
  CommandSuccess = 200,
  DeviceConnectInProgress = 201,
  DeviceConnectSuccess = 202,
  SearchStarted = 205,
  SearchStopped = 206,
  MalformedMessage = 400,
  DeviceConnectionFailed = 402,
  DeviceDisconnected = 403,
  DeviceNotFound = 404,
  DongleScanningInterruption = 501,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum LovenseDongleMessageType {
  #[allow(clippy::upper_case_acronyms)]
  #[serde(rename = "usb")]
  Usb,
  #[serde(rename = "toy")]
  Toy,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LovenseDongleMessageFunc {
  #[serde(rename = "reset")]
  Reset,
  #[serde(rename = "init")]
  Init,
  #[serde(rename = "search")]
  Search,
  #[serde(rename = "stopSearch")]
  StopSearch,
  #[serde(rename = "status")]
  IncomingStatus,
  #[serde(rename = "command")]
  Command,
  #[serde(rename = "toyData")]
  ToyData,
  #[serde(rename = "connect")]
  Connect,
  #[serde(rename = "error")]
  Error,
  #[serde(rename = "statuss")]
  Statuss,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LovenseDongleOutgoingMessage {
  #[serde(rename = "type")]
  pub message_type: LovenseDongleMessageType,
  pub func: LovenseDongleMessageFunc,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(rename = "cmd", skip_serializing_if = "Option::is_none")]
  pub command: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub eager: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LovenseDongleIncomingData {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub status: Option<LovenseDongleResultCode>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LovenseDongleIncomingMessage {
  #[serde(rename = "type")]
  pub message_type: LovenseDongleMessageType,
  pub func: LovenseDongleMessageFunc,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub id: Option<String>,
  #[serde(rename = "cmd", skip_serializing_if = "Option::is_none")]
  pub command: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub eager: Option<u32>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub result: Option<LovenseDongleResultCode>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub data: Option<LovenseDongleIncomingData>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub message: Option<String>,
}
