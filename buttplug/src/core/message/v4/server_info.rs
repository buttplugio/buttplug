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
pub struct ServerInfoV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ApiVersionMajor"))]
  #[getset(get_copy = "pub")]
  api_version_major: ButtplugMessageSpecVersion,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ApiVersionMinor"))]
  #[getset(get_copy = "pub")]
  api_version_minor: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "MaxPingTime"))]
  #[getset(get_copy = "pub")]
  max_ping_time: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ServerName"))]
  #[getset(get = "pub")]
  server_name: String,
}

impl ServerInfoV4 {
  pub fn new(
    server_name: &str,
    api_version_major: ButtplugMessageSpecVersion,
    api_version_minor: u32,
    max_ping_time: u32,
  ) -> Self {
    Self {
      id: 1,
      api_version_major,
      api_version_minor,
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
