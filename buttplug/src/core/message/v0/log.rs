// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::log_level::LogLevel;
use crate::core::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Log message received from server (Version 1 Message, Deprecated)
#[derive(
  Debug, ButtplugMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LogV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "LogLevel"))]
  #[getset(get_copy = "pub")]
  log_level: LogLevel,
  #[cfg_attr(feature = "serialize-json", serde(rename = "LogMessage"))]
  #[getset(get = "pub")]
  log_message: String,
}

impl LogV0 {
  pub fn new(log_level: LogLevel, log_message: &str) -> Self {
    Self {
      id: 0,
      log_level,
      log_message: log_message.to_owned(),
    }
  }
}

impl ButtplugMessageValidator for LogV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}
