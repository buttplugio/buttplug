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

fn return_version0() -> ButtplugMessageSpecVersion {
  ButtplugMessageSpecVersion::Version0
}

// For RequestServerInfo, serde will take care of invalid message versions from json, and internal
// representations of versions require using the version enum as a type bound. Therefore we do not
// need explicit content checking for the message.
#[derive(
  Debug, ButtplugMessage, ButtplugMessageFinalizer, Clone, PartialEq, Eq, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct RequestServerInfoV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ClientName"))]
  #[getset(get = "pub")]
  client_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ApiVersionMajor"))]
  #[getset(get_copy = "pub")]
  api_version_major: ButtplugMessageSpecVersion,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ApiVersionMinor"))]
  #[getset(get_copy = "pub")]
  api_version_minor: u32,  
}

impl RequestServerInfoV4 {
  pub fn new(client_name: &str, api_version_major: ButtplugMessageSpecVersion, api_version_minor: u32) -> Self {
    Self {
      id: 1,
      client_name: client_name.to_string(),
      api_version_major,
      api_version_minor
    }
  }
}

impl ButtplugMessageValidator for RequestServerInfoV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
