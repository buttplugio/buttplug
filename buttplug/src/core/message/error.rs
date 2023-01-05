// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Notification of an error in the system, due to a failed external command or internal failure

use super::*;
use crate::core::errors::*;
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize-json")]
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Error codes pertaining to error classes that can be represented in the
/// Buttplug [Error] message.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "serialize-json", derive(Serialize_repr, Deserialize_repr))]
#[repr(u8)]
pub enum ErrorCode {
  ErrorUnknown = 0,
  ErrorHandshake,
  ErrorPing,
  ErrorMessage,
  ErrorDevice,
}

/// Represents the Buttplug Protocol Error message, as documented in the [Buttplug
/// Protocol Spec](https://buttplug-spec.docs.buttplug.io/status.html#error).
// Error is one of the few things that can have either a System ID or message
// ID, so there's really not much to check here. Use the default trait impl for
// ButtplugMessageValidator.
#[derive(
  Debug,
  Clone,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  Getters,
  CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct Error {
  /// Message Id, used for matching message pairs in remote connection instances.
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  /// Specifies the class of the error.
  #[cfg_attr(feature = "serialize-json", serde(rename = "ErrorCode"))]
  #[getset(get_copy = "pub")]
  error_code: ErrorCode,
  /// Description of the error.
  #[cfg_attr(feature = "serialize-json", serde(rename = "ErrorMessage"))]
  #[getset(get = "pub")]
  error_message: String,
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  original_error: Option<ButtplugError>,
}

impl PartialEq for Error {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
      && self.error_code == other.error_code
      && self.error_message == other.error_message
  }
}

impl Error {
  /// Creates a new error object.
  pub fn new(
    error_code: ErrorCode,
    error_message: &str,
    original_error: Option<ButtplugError>,
  ) -> Self {
    Self {
      id: 0,
      error_code,
      error_message: error_message.to_string(),
      original_error,
    }
  }

  pub fn original_error(&self) -> ButtplugError {
    if self.original_error.is_some() {
      self
        .original_error
        .clone()
        .expect("Already checked that it's valid.")
    } else {
      // Try deserializing what's in the error_message field
      #[cfg(feature = "serialize-json")]
      {
        if let Ok(deserialized_msg) = serde_json::from_str(&self.error_message) {
          return deserialized_msg;
        }
      }
      ButtplugError::from(self.clone())
    }
  }
}

impl From<ButtplugError> for Error {
  /// Converts a [ButtplugError] object into a Buttplug Protocol
  /// [Error] message.
  fn from(error: ButtplugError) -> Self {
    let code = match error {
      ButtplugError::ButtplugDeviceError { .. } => ErrorCode::ErrorDevice,
      ButtplugError::ButtplugMessageError { .. } => ErrorCode::ErrorMessage,
      ButtplugError::ButtplugPingError { .. } => ErrorCode::ErrorPing,
      ButtplugError::ButtplugHandshakeError { .. } => ErrorCode::ErrorHandshake,
      ButtplugError::ButtplugUnknownError { .. } => ErrorCode::ErrorUnknown,
    };
    #[cfg(feature = "serialize-json")]
    let msg = serde_json::to_string(&error).expect("All buttplug errors are serializable");
    #[cfg(not(feature = "serialize-json"))]
    let msg = error.to_string();
    Error::new(code, &msg, Some(error))
  }
}

#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  ButtplugMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  Getters,
  CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ErrorV0 {
  /// Message Id, used for matching message pairs in remote connection instances.
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  /// Specifies the class of the error.
  #[cfg_attr(feature = "serialize-json", serde(rename = "ErrorCode"))]
  #[getset(get_copy = "pub")]
  error_code: ErrorCode,
  /// Description of the error.
  #[cfg_attr(feature = "serialize-json", serde(rename = "ErrorMessage"))]
  #[getset(get = "pub")]
  error_message: String,
}

impl ErrorV0 {
  /// Creates a new error object.
  pub fn new(error_code: ErrorCode, error_message: &str) -> Self {
    Self {
      id: 0,
      error_code,
      error_message: error_message.to_string(),
    }
  }
}

impl From<Error> for ErrorV0 {
  fn from(error: Error) -> Self {
    let mut err = ErrorV0::new(error.error_code, &error.error_message);
    err.set_id(error.id());
    err
  }
}

#[cfg(feature = "serialize-json")]
#[cfg(test)]
mod test {
  use crate::core::message::{ButtplugCurrentSpecServerMessage, Error, ErrorCode};

  const ERROR_STR: &str = "{\"Error\":{\"Id\":0,\"ErrorCode\":1,\"ErrorMessage\":\"Test Error\"}}";

  #[test]
  fn test_error_serialize() {
    let error = ButtplugCurrentSpecServerMessage::Error(Error::new(
      ErrorCode::ErrorHandshake,
      "Test Error",
      None,
    ));
    let js = serde_json::to_string(&error).expect("Infallible serialization.");
    assert_eq!(ERROR_STR, js);
  }

  #[test]
  fn test_error_deserialize() {
    let union: ButtplugCurrentSpecServerMessage =
      serde_json::from_str(ERROR_STR).expect("Infallible deserialization");
    assert_eq!(
      ButtplugCurrentSpecServerMessage::Error(Error::new(
        ErrorCode::ErrorHandshake,
        "Test Error",
        None
      )),
      union
    );
  }
}
