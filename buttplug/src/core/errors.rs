// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Error Structs/Enums, representing protocol errors.

use super::messages::{
  self,
  serializer::ButtplugSerializerError,
  ButtplugDeviceMessageType,
  ButtplugMessage,
  ButtplugMessageSpecVersion,
  ErrorCode,
};
use crate::{device::Endpoint, server::comm_managers::ButtplugDeviceSpecificError};
use displaydoc::Display;
use futures::future::{self, BoxFuture};
use std::fmt;
use thiserror::Error;

pub type ButtplugResult<T = ()> = Result<T, ButtplugError>;

/// Handshake errors occur while a client is connecting to a server. This
/// usually involves protocol handshake errors. For connector errors (i.e. when
/// a remote network connection cannot be established), see
/// [crate::connector::ButtplugClientConnectorError].
impl<T> From<ButtplugHandshakeError> for BoxFuture<'static, Result<T, ButtplugError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugHandshakeError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

#[derive(Debug, Error, Display, Clone)]
pub enum ButtplugHandshakeError {
  /// Expected either a ServerInfo or Error message, received {0}
  UnexpectedHandshakeMessageReceived(String),
  /// Expected a RequestServerInfo message to start connection. Message either not received or wrong message received.
  RequestServerInfoExpected,
  /// Handshake already happened, cannot run handshake again.
  HandshakeAlreadyHappened,
  /// Server spec version ({0}) must be equal or greater than client version ({1})
  MessageSpecVersionMismatch(ButtplugMessageSpecVersion, ButtplugMessageSpecVersion),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Message errors occur when a message is somehow malformed on creation, or
/// received unexpectedly by a client or server.
impl<T> From<ButtplugMessageError> for BoxFuture<'static, Result<T, ButtplugError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugMessageError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

#[derive(Debug, Error, Display, Clone)]
pub enum ButtplugMessageError {
  /// Got unexpected message type: {0}
  UnexpectedMessageType(String),
  /// {0} {1} cannot be converted to {2}
  VersionError(&'static str, String, &'static str),
  /// Message conversion error: {0}
  MessageConversionError(&'static str),
  /// Unhandled message type: {0}
  UnhandledMessage(String),
  /// Message validation error(s): {0}
  ValidationError(&'static str),
  /// Message serialization error
  #[error(transparent)]
  MessageSerializationError(#[from] ButtplugSerializerError),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Ping errors occur when a server requires a ping response (set up during
/// connection handshake), and the client does not return a response in the
/// alloted timeframe. This also signifies a server disconnect.
impl<T> From<ButtplugPingError> for BoxFuture<'static, Result<T, ButtplugError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugPingError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

#[derive(Debug, Error, Display, Clone)]
pub enum ButtplugPingError {
  /// Pinged timer exhausted, system has shut down.
  PingedOut,
  /// Ping timer not running.
  PingTimerNotRunning,
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Device errors occur during device interactions, including sending
/// unsupported message commands, addressing the wrong number of device
/// attributes, etc...
impl<T> From<ButtplugDeviceError> for BoxFuture<'static, Result<T, ButtplugError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugDeviceError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}
#[derive(Debug, Error, Display, Clone)]
pub enum ButtplugDeviceError {
  /// Device {0} not connected
  DeviceNotConnected(String),
  /// Device does not support message type {0}.
  MessageNotSupported(ButtplugDeviceMessageType),
  /// Device only has {0} features, but {1} commands were sent.
  DeviceFeatureCountMismatch(u32, u32),
  /// Device only has {0} features, but was given an index of {1}
  DeviceFeatureIndexError(u32, u32),
  /// Device connection error: {0}
  DeviceConnectionError(String),
  /// Device communication error: {0}
  DeviceCommunicationError(String),
  /// Device does not have endpoint {0}
  InvalidEndpoint(Endpoint),
  /// Device does not handle command type: {0}
  UnhandledCommand(String),
  /// Device type specific error.
  #[error(transparent)]
  DeviceSpecificError(#[from] ButtplugDeviceSpecificError),
  /// No device available at index {0}
  DeviceNotAvailable(u32),
  /// Device scanning already started.
  DeviceScanningAlreadyStarted,
  /// Device scanning already stopped.
  DeviceScanningAlreadyStopped,
  /// Device permission error: {0}
  DevicePermissionError(String),
  /// {0}
  ProtocolAttributesNotFound(String),
  /// Protocol {0} not implemented in library
  ProtocolNotImplemented(String),
  /// {0} protocol specific error: {1}
  ProtocolSpecificError(&'static str, &'static str),
  /// {0}
  ProtocolRequirementError(String),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Unknown errors occur in exceptional circumstances where no other error type
/// will suffice. These are rare and usually fatal (disconnecting) errors.
impl<T> From<ButtplugUnknownError> for BoxFuture<'static, Result<T, ButtplugError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugUnknownError) -> BoxFuture<'static, Result<T, ButtplugError>> {
    ButtplugError::from(err).into()
  }
}

#[derive(Debug, Error, Display, Clone)]
pub enum ButtplugUnknownError {
  /// Cannot start scanning, no device communication managers available to use for scanning.
  NoDeviceCommManagers,
  /// Got unexpected enum type: {0}
  UnexpectedType(String),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Aggregation enum for protocol error types.
#[derive(Debug, Error, Clone)]
pub enum ButtplugError {
  #[error(transparent)]
  ButtplugHandshakeError(#[from] ButtplugHandshakeError),
  #[error(transparent)]
  ButtplugMessageError(#[from] ButtplugMessageError),
  #[error(transparent)]
  ButtplugPingError(#[from] ButtplugPingError),
  #[error(transparent)]
  ButtplugDeviceError(#[from] ButtplugDeviceError),
  #[error(transparent)]
  ButtplugUnknownError(#[from] ButtplugUnknownError),
}

impl From<ButtplugServerError> for ButtplugError {
  fn from(error: ButtplugServerError) -> Self {
    error.error().clone()
  }
}

#[derive(Debug, Display, Clone)]
pub struct ButtplugServerError {
  msg_id: u32,
  error: ButtplugError,
}

impl ButtplugServerError {
  pub fn new_message_error(msg_id: u32, error: ButtplugError) -> Self {
    Self { msg_id, error }
  }

  pub fn new_system_error(error: ButtplugError) -> Self {
    Self { msg_id: 0, error }
  }

  pub fn id(&self) -> u32 {
    self.msg_id
  }

  pub fn error(&self) -> &ButtplugError {
    &self.error
  }
}

impl From<ButtplugError> for ButtplugServerError {
  fn from(error: ButtplugError) -> Self {
    ButtplugServerError::new_system_error(error)
  }
}

impl From<ButtplugMessageError> for ButtplugServerError {
  fn from(err: ButtplugMessageError) -> Self {
    ButtplugServerError::new_system_error(err.into())
  }
}

impl From<ButtplugUnknownError> for ButtplugServerError {
  fn from(err: ButtplugUnknownError) -> Self {
    ButtplugServerError::new_system_error(err.into())
  }
}

impl From<ButtplugDeviceError> for ButtplugServerError {
  fn from(err: ButtplugDeviceError) -> Self {
    ButtplugServerError::new_system_error(err.into())
  }
}

impl From<ButtplugPingError> for ButtplugServerError {
  fn from(err: ButtplugPingError) -> Self {
    ButtplugServerError::new_system_error(err.into())
  }
}

impl From<ButtplugHandshakeError> for ButtplugServerError {
  fn from(err: ButtplugHandshakeError) -> Self {
    ButtplugServerError::new_system_error(err.into())
  }
}

impl std::fmt::Display for ButtplugServerError {
  // This trait requires `fmt` with this exact signature.
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.error.fmt(f)
  }
}

impl std::error::Error for ButtplugServerError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    self.error.source()
  }
}

impl From<messages::Error> for ButtplugServerError {
  /// Turns a Buttplug Protocol Error Message [super::messages::Error] into a [ButtplugError] type.
  fn from(error: messages::Error) -> Self {
    ButtplugServerError::new_message_error(error.get_id(), ButtplugError::from(error))
  }
}

impl<T> From<ButtplugServerError> for BoxFuture<'static, Result<T, ButtplugServerError>>
where
  T: Send + 'static,
{
  fn from(err: ButtplugServerError) -> BoxFuture<'static, Result<T, ButtplugServerError>> {
    Box::pin(future::ready(Err(err)))
  }
}

impl From<messages::Error> for ButtplugError {
  /// Turns a Buttplug Protocol Error Message [super::messages::Error] into a [ButtplugError] type.
  fn from(error: messages::Error) -> Self {
    match error.error_code {
      ErrorCode::ErrorDevice => {
        ButtplugDeviceError::UntypedDeserializedError(error.error_message).into()
      }
      ErrorCode::ErrorMessage => {
        ButtplugMessageError::UntypedDeserializedError(error.error_message).into()
      }
      ErrorCode::ErrorHandshake => {
        ButtplugHandshakeError::UntypedDeserializedError(error.error_message).into()
      }
      ErrorCode::ErrorUnknown => {
        ButtplugUnknownError::UntypedDeserializedError(error.error_message).into()
      }
      ErrorCode::ErrorPing => {
        ButtplugPingError::UntypedDeserializedError(error.error_message).into()
      }
    }
  }
}
