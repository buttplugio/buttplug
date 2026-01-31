// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Error Structs/Enums, representing protocol errors.

use super::message::{
  self,
  ButtplugMessageSpecVersion,
  ErrorCode,
  InputType,
  OutputType,
  serializer::ButtplugSerializerError,
};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type ButtplugResult<T = ()> = Result<T, ButtplugError>;

/// Macro for implementing `From<ErrorType> for BoxFuture<'static, Result<T, ButtplugError>>`.
/// These implementations allow error types to be converted directly into ready futures.
macro_rules! impl_error_to_future {
  ($($error_type:ty),* $(,)?) => {
    $(
      impl<T> From<$error_type> for BoxFuture<'static, Result<T, ButtplugError>>
      where
        T: Send + 'static,
      {
        fn from(err: $error_type) -> BoxFuture<'static, Result<T, ButtplugError>> {
          ButtplugError::from(err).into()
        }
      }
    )*
  };
}

impl_error_to_future!(
  ButtplugHandshakeError,
  ButtplugMessageError,
  ButtplugPingError,
  ButtplugDeviceError,
  ButtplugUnknownError,
);

/// Handshake errors occur while a client is connecting to a server. This
/// usually involves protocol handshake errors. For connector errors (i.e. when
/// a remote network connection cannot be established), see
/// [crate::connector::ButtplugConnectorError].

#[derive(Debug, Error, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugHandshakeError {
  /// Expected either a ServerInfo or Error message, received {0}
  UnexpectedHandshakeMessageReceived(String),
  /// Expected a RequestServerInfo message to start connection. Message either not received or wrong message received.
  RequestServerInfoExpected,
  /// Handshake already happened, cannot run handshake again.
  HandshakeAlreadyHappened,
  /// Server has already connected and disconnected, cannot be reused
  ReconnectDenied,
  /// Server spec version ({0}) must be equal or greater than client version ({1})
  MessageSpecVersionMismatch(ButtplugMessageSpecVersion, ButtplugMessageSpecVersion),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
  /// Unhandled spec version requested, may require extra arguments to activate: {0}
  UnhandledMessageSpecVersionRequested(ButtplugMessageSpecVersion),
}

/// Message errors occur when a message is somehow malformed on creation, or
/// received unexpectedly by a client or server.
#[derive(Debug, Error, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugMessageError {
  /// Got unexpected message type: {0}
  UnexpectedMessageType(String),
  /// {0} {1} cannot be converted to {2}
  VersionError(String, String, String),
  /// Message conversion error: {0}
  MessageConversionError(String),
  /// Invalid message contents: {0}
  InvalidMessageContents(String),
  /// Unhandled message type: {0}
  UnhandledMessage(String),
  /// Message validation error(s): {0}
  ValidationError(String),
  /// Message serialization error
  #[error(transparent)]
  MessageSerializationError(#[from] ButtplugSerializerError),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Ping errors occur when a server requires a ping response (set up during
/// connection handshake), and the client does not return a response in the
/// alloted timeframe. This also signifies a server disconnect.
#[derive(Debug, Error, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugPingError {
  /// Pinged timer exhausted, system has shut down.
  PingedOut,
  /// Ping timer not running.
  PingTimerNotRunning,
  /// Ping time must be greater than 0.
  InvalidPingTimeout,
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
}

/// Device errors occur during device interactions, including sending
/// unsupported message commands, addressing the wrong number of device
/// attributes, etc...
#[derive(Debug, Error, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugDeviceError {
  /// Device {0} not connected
  DeviceNotConnected(String),
  /// Device does not support message type {0}.
  MessageNotSupported(String),
  /// Device only has {0} features, but {1} commands were sent.
  DeviceFeatureCountMismatch(u32, u32),
  /// Device only has {0} features, but was given an index of {1}
  DeviceFeatureIndexError(u32, u32),
  /// Device feature mismatch: {0}
  DeviceFeatureMismatch(String),
  /// Device only has {0} sensors, but was given an index of {1}
  DeviceSensorIndexError(u32, u32),
  /// Device connection error: {0}
  DeviceConnectionError(String),
  /// Device communication error: {0}
  DeviceCommunicationError(String),
  /// Device feature only has {0} steps for control, but {1} steps specified.
  DeviceStepRangeError(i32, i32),
  /// Device got {} output command but has no viable outputs
  DeviceNoOutputError(OutputType),
  /// Device got {0} input command but has no viable inputs
  DeviceNoInputError(InputType),
  /// Device does not have endpoint {0}
  InvalidEndpoint(String),
  /// Device does not handle command type: {0}
  UnhandledCommand(String),
  /// Device type specific error: {0}.
  DeviceSpecificError(String),
  /// No device available at index {0}
  DeviceNotAvailable(u32),
  /// Device scanning already started.
  DeviceScanningAlreadyStarted,
  /// Device scanning already stopped.
  DeviceScanningAlreadyStopped,
  /// Device permission error: {0}
  DevicePermissionError(String),
  /// Device command does not take negative numbers
  DeviceCommandSignError,
  /// {0}
  ProtocolAttributesNotFound(String),
  /// Protocol {0} not implemented in library
  ProtocolNotImplemented(String),
  /// {0} protocol specific error: {1}
  ProtocolSpecificError(String, String),
  /// {0}
  ProtocolRequirementError(String),
  /// Protocol already added to system {0},
  ProtocolAlreadyAdded(String),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
  /// Device Configuration Error: {0}
  DeviceConfigurationError(String),
  /// Output Type Mismatch: Index {0} got command for {1}, which is not valid
  DeviceOutputTypeMismatch(u32, OutputType, OutputType),
  /// Input Type Mismatch: Index {0} got command for {1}, which is not valid
  DeviceInputTypeMismatch(u32, InputType),
  /// Protocol does not have an implementation available for Sensor Type {0}
  ProtocolInputNotSupported(InputType),
  /// Device does not support {0}
  OutputNotSupported(OutputType),
}

/// Unknown errors occur in exceptional circumstances where no other error type
/// will suffice. These are rare and usually fatal (disconnecting) errors.
#[derive(Debug, Error, Display, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtplugUnknownError {
  /// Cannot start scanning, no device communication managers available to use for scanning.
  NoDeviceCommManagers,
  /// Got unexpected enum type: {0}
  UnexpectedType(String),
  /// Untyped Deserialized Error: {0}
  UntypedDeserializedError(String),
  /// Device Manager has been shut down by its owning server and is no longer available.
  DeviceManagerNotRunning,
}

/// Aggregation enum for protocol error types.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl From<message::ErrorV0> for ButtplugError {
  /// Turns a Buttplug Protocol Error Message [super::messages::Error] into a [ButtplugError] type.
  fn from(error: message::ErrorV0) -> Self {
    match error.error_code() {
      ErrorCode::ErrorDevice => {
        ButtplugDeviceError::UntypedDeserializedError(error.error_message().clone()).into()
      }
      ErrorCode::ErrorMessage => {
        ButtplugMessageError::UntypedDeserializedError(error.error_message().clone()).into()
      }
      ErrorCode::ErrorHandshake => {
        ButtplugHandshakeError::UntypedDeserializedError(error.error_message().clone()).into()
      }
      ErrorCode::ErrorUnknown => {
        ButtplugUnknownError::UntypedDeserializedError(error.error_message().clone()).into()
      }
      ErrorCode::ErrorPing => {
        ButtplugPingError::UntypedDeserializedError(error.error_message().clone()).into()
      }
    }
  }
}
