// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use std::cmp::Ord;
use tracing::Level;

#[derive(Debug, PartialEq, Clone, Ord, PartialOrd, Eq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub enum LogLevel {
  Off = 0,
  Fatal,
  Error,
  Warn,
  Info,
  Debug,
  Trace,
}

impl From<Level> for LogLevel {
  fn from(level: Level) -> Self {
    match level {
      Level::ERROR => LogLevel::Error,
      Level::WARN => LogLevel::Warn,
      Level::INFO => LogLevel::Info,
      Level::DEBUG => LogLevel::Debug,
      Level::TRACE => LogLevel::Trace,
    }
  }
}

impl Into<Level> for LogLevel {
  fn into(self) -> Level {
    match self {
      // Rust doesn't have a Fatal level, and we never use it in code, so
      // just convert to Error.
      LogLevel::Fatal => Level::ERROR,
      LogLevel::Error => Level::ERROR,
      LogLevel::Warn => Level::WARN,
      LogLevel::Info => Level::INFO,
      LogLevel::Debug => Level::DEBUG,
      LogLevel::Trace => Level::TRACE,
      LogLevel::Off => {
        error!("Log messages with a log level of Off are not allowed");
        Level::ERROR
      }
    }
  }
}
