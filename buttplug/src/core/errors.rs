// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Error Structs/Enums, representing protocol errors.

use super::messages::{self, ErrorCode};
use std::error::Error;
use std::fmt;
use futures::future::BoxFuture;

pub type ButtplugResult<T = ()> = Result<T, ButtplugError>;

/// Handshake errors occur while a client is connecting to a server. This
/// usually involves protocol handshake errors. For connector errors (i.e. when
/// a remote network connection cannot be established), see
/// [crate::connector::ButtplugClientConnectorError].
#[derive(Debug, Clone)]
pub struct ButtplugHandshakeError {
  /// Message for the handshake error.
  pub message: String,
}

impl ButtplugHandshakeError {
  pub fn new(message: &str) -> Self {
    Self {
      message: message.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugHandshakeError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Init Error: {}", self.message)
  }
}

impl Error for ButtplugHandshakeError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugHandshakeError> for BoxFuture<'static, Result<T, ButtplugError>> where T: Send + 'static {
  fn from(err: ButtplugHandshakeError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

/// Message errors occur when a message is somehow malformed on creation, or
/// received unexpectedly by a client or server.
#[derive(Debug, Clone)]
pub struct ButtplugMessageError {
  pub message: String,
}

impl ButtplugMessageError {
  pub fn new(message: &str) -> Self {
    Self {
      message: message.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugMessageError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Message Error: {}", self.message)
  }
}

impl Error for ButtplugMessageError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugMessageError> for BoxFuture<'static, Result<T, ButtplugError>> where T: Send + 'static {
  fn from(err: ButtplugMessageError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

/// Ping errors occur when a server requires a ping response (set up during
/// connection handshake), and the client does not return a response in the
/// alloted timeframe. This also signifies a server disconnect.
#[derive(Debug, Clone)]
pub struct ButtplugPingError {
  pub message: String,
}

impl ButtplugPingError {
  pub fn new(message: &str) -> Self {
    Self {
      message: message.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugPingError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Ping Error: {}", self.message)
  }
}

impl Error for ButtplugPingError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugPingError> for BoxFuture<'static, Result<T, ButtplugError>> where T: Send + 'static {
  fn from(err: ButtplugPingError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

/// Device errors occur during device interactions, including sending
/// unsupported message commands, addressing the wrong number of device
/// attributes, etc...
#[derive(Debug, Clone)]
pub struct ButtplugDeviceError {
  pub message: String,
}

impl ButtplugDeviceError {
  pub fn new(message: &str) -> Self {
    Self {
      message: message.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugDeviceError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Device Error: {}", self.message)
  }
}

impl Error for ButtplugDeviceError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugDeviceError> for BoxFuture<'static, Result<T, ButtplugError>> where T: Send + 'static {
  fn from(err: ButtplugDeviceError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

/// Unknown errors occur in exceptional circumstances where no other error type
/// will suffice. These are rare and usually fatal (disconnecting) errors.
#[derive(Debug, Clone)]
pub struct ButtplugUnknownError {
  pub message: String,
}

impl ButtplugUnknownError {
  pub fn new(message: &str) -> Self {
    Self {
      message: message.to_owned(),
    }
  }
}

impl fmt::Display for ButtplugUnknownError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Unknown Error: {}", self.message)
  }
}

impl Error for ButtplugUnknownError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl<T> From<ButtplugUnknownError> for BoxFuture<'static, Result<T, ButtplugError>> where T: Send + 'static {
  fn from(err: ButtplugUnknownError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

/// Aggregation enum for protocol error types.
#[derive(Debug, Clone)]
pub enum ButtplugError {
  ButtplugHandshakeError(ButtplugHandshakeError),
  ButtplugMessageError(ButtplugMessageError),
  ButtplugPingError(ButtplugPingError),
  ButtplugDeviceError(ButtplugDeviceError),
  ButtplugUnknownError(ButtplugUnknownError),
}

impl fmt::Display for ButtplugError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      ButtplugError::ButtplugDeviceError(ref e) => e.fmt(f),
      ButtplugError::ButtplugMessageError(ref e) => e.fmt(f),
      ButtplugError::ButtplugPingError(ref e) => e.fmt(f),
      ButtplugError::ButtplugHandshakeError(ref e) => e.fmt(f),
      ButtplugError::ButtplugUnknownError(ref e) => e.fmt(f),
    }
  }
}

impl Error for ButtplugError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl From<ButtplugDeviceError> for ButtplugError {
  fn from(error: ButtplugDeviceError) -> Self {
    ButtplugError::ButtplugDeviceError(error)
  }
}

impl From<ButtplugMessageError> for ButtplugError {
  fn from(error: ButtplugMessageError) -> Self {
    ButtplugError::ButtplugMessageError(error)
  }
}

impl From<ButtplugPingError> for ButtplugError {
  fn from(error: ButtplugPingError) -> Self {
    ButtplugError::ButtplugPingError(error)
  }
}

impl From<ButtplugHandshakeError> for ButtplugError {
  fn from(error: ButtplugHandshakeError) -> Self {
    ButtplugError::ButtplugHandshakeError(error)
  }
}

impl From<ButtplugUnknownError> for ButtplugError {
  fn from(error: ButtplugUnknownError) -> Self {
    ButtplugError::ButtplugUnknownError(error)
  }
}

impl From<messages::Error> for ButtplugError {
  /// Turns a Buttplug Protocol Error Message [super::messages::Error] into a [ButtplugError] type.
  fn from(error: messages::Error) -> Self {
    match error.error_code {
      ErrorCode::ErrorDevice => ButtplugError::ButtplugDeviceError(ButtplugDeviceError {
        message: error.error_message,
      }),
      ErrorCode::ErrorMessage => ButtplugError::ButtplugMessageError(ButtplugMessageError {
        message: error.error_message,
      }),
      ErrorCode::ErrorHandshake => ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError {
        message: error.error_message,
      }),
      ErrorCode::ErrorUnknown => ButtplugError::ButtplugUnknownError(ButtplugUnknownError {
        message: error.error_message,
      }),
      ErrorCode::ErrorPing => ButtplugError::ButtplugPingError(ButtplugPingError {
        message: error.error_message,
      }),
    }
  }
}
