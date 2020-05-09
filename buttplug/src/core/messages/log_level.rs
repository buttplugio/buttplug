// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use log;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub enum LogLevel {
  Off = 0,
  Fatal,
  Error,
  Warn,
  Info,
  Debug,
  Trace,
}

impl From<log::Level> for LogLevel {
  fn from(level: log::Level) -> Self {
    match level {
      log::Level::Error => LogLevel::Error,
      log::Level::Warn => LogLevel::Warn,
      log::Level::Info => LogLevel::Info,
      log::Level::Debug => LogLevel::Debug,
      log::Level::Trace => LogLevel::Trace,
    }
  }
}

impl Into<log::Level> for LogLevel {
  fn into(self) -> log::Level {
    match self {
      // Rust doesn't have a Fatal level, and we never use it in code, so
      // just convert to Error.
      LogLevel::Fatal => log::Level::Error,
      LogLevel::Error => log::Level::Error,
      LogLevel::Warn => log::Level::Warn,
      LogLevel::Info => log::Level::Info,
      LogLevel::Debug => log::Level::Debug,
      LogLevel::Trace => log::Level::Trace,
      LogLevel::Off => panic!("Log messages with a log level of Off are not allowed"),
    }
  }
}

impl Into<log::LevelFilter> for LogLevel {
  fn into(self) -> log::LevelFilter {
    match self {
      // Rust doesn't have a Fatal level, and we never use it in code, so
      // just convert to Error.
      LogLevel::Fatal => log::LevelFilter::Error,
      LogLevel::Error => log::LevelFilter::Error,
      LogLevel::Warn => log::LevelFilter::Warn,
      LogLevel::Info => log::LevelFilter::Info,
      LogLevel::Debug => log::LevelFilter::Debug,
      LogLevel::Trace => log::LevelFilter::Trace,
      LogLevel::Off => log::LevelFilter::Off,
    }
  }
}
