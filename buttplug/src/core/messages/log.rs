// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Log {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "LogLevel"))]
  pub log_level: LogLevel,
  #[cfg_attr(feature = "serialize_json", serde(rename = "LogMessage"))]
  pub log_message: String,
}

impl Log {
  pub fn new(log_level: LogLevel, log_message: &str) -> Self {
    Self {
      id: 0,
      log_level,
      log_message: log_message.to_owned(),
    }
  }
}
