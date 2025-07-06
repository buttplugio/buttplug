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

// For RequestServerInfo, serde will take care of invalid message versions from json, and internal
// representations of versions require using the version enum as a type bound. Therefore we do not
// need explicit content checking for the message.
#[derive(
  Debug,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  Clone,
  PartialEq,
  Eq,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct RequestServerInfoV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "ClientName")]
  #[getset(get = "pub")]
  client_name: String,
  #[serde(rename = "ProtocolVersionMajor")]
  #[getset(get_copy = "pub")]
  protocol_version_major: ButtplugMessageSpecVersion,
  #[serde(rename = "ProtocolVersionMinor")]
  #[getset(get_copy = "pub")]
  protocol_version_minor: u32,
}

impl RequestServerInfoV4 {
  pub fn new(
    client_name: &str,
    protocol_version_major: ButtplugMessageSpecVersion,
    protocol_version_minor: u32,
  ) -> Self {
    Self {
      id: 1,
      client_name: client_name.to_string(),
      protocol_version_major,
      protocol_version_minor,
    }
  }
}

impl ButtplugMessageValidator for RequestServerInfoV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
