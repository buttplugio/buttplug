// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageSpecVersion,
  ButtplugMessageValidator,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct ServerInfoV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "ProtocolVersionMajor")]
  #[getset(get_copy = "pub")]
  protocol_version_major: ButtplugMessageSpecVersion,
  #[serde(rename = "ProtocolVersionMinor")]
  #[getset(get_copy = "pub")]
  protocol_version_minor: u32,
  #[serde(rename = "MaxPingTime")]
  #[getset(get_copy = "pub")]
  max_ping_time: u32,
  #[serde(rename = "ServerName")]
  #[getset(get = "pub")]
  server_name: String,
}

impl ServerInfoV4 {
  pub fn new(
    server_name: &str,
    protocol_version_major: ButtplugMessageSpecVersion,
    protocol_version_minor: u32,
    max_ping_time: u32,
  ) -> Self {
    Self {
      id: 1,
      protocol_version_major,
      protocol_version_minor,
      max_ping_time,
      server_name: server_name.to_string(),
    }
  }
}

impl ButtplugMessageValidator for ServerInfoV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
