// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ServerInfo {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MessageVersion"))]
  pub message_version: ButtplugMessageSpecVersion,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MaxPingTime"))]
  pub max_ping_time: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ServerName"))]
  pub server_name: String,
}

impl ServerInfo {
  pub fn new(
    server_name: &str,
    message_version: ButtplugMessageSpecVersion,
    max_ping_time: u32,
  ) -> Self {
    Self {
      id: 1,
      message_version,
      max_ping_time,
      server_name: server_name.to_string(),
    }
  }
}
#[derive(Debug, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ServerInfoV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MajorVersion"))]
  pub major_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MinorVersion"))]
  pub minor_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "BuildVersion"))]
  pub build_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MessageVersion"))]
  pub message_version: ButtplugMessageSpecVersion,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MaxPingTime"))]
  pub max_ping_time: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ServerName"))]
  pub server_name: String,
}

impl ServerInfoV0 {
  pub fn new(
    server_name: &str,
    message_version: ButtplugMessageSpecVersion,
    max_ping_time: u32,
  ) -> Self {
    Self {
      id: 1,
      major_version: 0,
      minor_version: 0,
      build_version: 0,
      message_version,
      max_ping_time,
      server_name: server_name.to_string(),
    }
  }
}

impl From<ServerInfo> for ServerInfoV0 {
  fn from(msg: ServerInfo) -> Self {
    let mut out_msg = Self::new(&msg.server_name, msg.message_version, msg.max_ping_time);
    out_msg.set_id(msg.get_id());
    out_msg
  }
}
