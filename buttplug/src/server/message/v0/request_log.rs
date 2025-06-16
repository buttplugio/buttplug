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
use getset::CopyGetters;
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct RequestLogV0 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "LogLevel")]
  #[getset(get_copy = "pub")]
  log_level: LogLevel,
}

impl RequestLogV0 {
  pub fn new(log_level: LogLevel) -> Self {
    Self { id: 1, log_level }
  }
}

impl ButtplugMessageValidator for RequestLogV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}
