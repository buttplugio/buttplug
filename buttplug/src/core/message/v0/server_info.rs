// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageSpecVersion,
  ButtplugMessageValidator,
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ServerInfoV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MajorVersion"))]
  #[getset(get_copy = "pub")]
  major_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MinorVersion"))]
  #[getset(get_copy = "pub")]
  minor_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "BuildVersion"))]
  #[getset(get_copy = "pub")]
  build_version: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MessageVersion"))]
  #[getset(get_copy = "pub")]
  message_version: ButtplugMessageSpecVersion,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MaxPingTime"))]
  #[getset(get_copy = "pub")]
  max_ping_time: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ServerName"))]
  #[getset(get = "pub")]
  server_name: String,
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

impl ButtplugMessageValidator for ServerInfoV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}
