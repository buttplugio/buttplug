// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::log_level::LogLevel;
use crate::core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

/// Log message received from server (Version 1 Message, Deprecated)
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
pub struct LogV0 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "LogLevel")]
  #[getset(get_copy = "pub")]
  log_level: LogLevel,
  #[serde(rename = "LogMessage")]
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
