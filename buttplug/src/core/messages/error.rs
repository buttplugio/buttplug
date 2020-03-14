// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use crate::core::errors::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serialize_json")]
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Error codes pertaining to error classes that can be represented in the
/// Buttplug [Error] message.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize_repr, Deserialize_repr))]
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
#[derive(
    Debug, 
    Clone,
    PartialEq,
    ButtplugMessage, 
    ToButtplugMessageUnion
)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct Error {
    /// Message Id, used for matching message pairs in remote connection instances.
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    /// Specifies the class of the error.
    #[cfg_attr(feature = "serialize_json", serde(rename = "ErrorCode"))]
    pub error_code: ErrorCode,
    /// Description of the error.
    #[cfg_attr(feature = "serialize_json", serde(rename = "ErrorMessage"))]
    pub error_message: String,
}

impl Error {
    /// Creates a new error object.
    pub fn new(error_code: ErrorCode, error_message: &str) -> Self {
        Self {
            id: 0,
            error_code,
            error_message: error_message.to_string(),
        }
    }
}

impl From<ButtplugError> for Error {
    /// Converts a [super::errors::ButtplugError] object into a Buttplug Protocol
    /// [Error] message.
    fn from(error: ButtplugError) -> Self {
        let code = match error {
            ButtplugError::ButtplugDeviceError(_) => ErrorCode::ErrorDevice,
            ButtplugError::ButtplugMessageError(_) => ErrorCode::ErrorMessage,
            ButtplugError::ButtplugPingError(_) => ErrorCode::ErrorPing,
            ButtplugError::ButtplugHandshakeError(_) => ErrorCode::ErrorHandshake,
            ButtplugError::ButtplugUnknownError(_) => ErrorCode::ErrorUnknown,
        };
        // Gross but was having problems with naming collisions on the error trait
        let msg = match error {
            ButtplugError::ButtplugDeviceError(_s) => _s.message,
            ButtplugError::ButtplugMessageError(_s) => _s.message,
            ButtplugError::ButtplugPingError(_s) => _s.message,
            ButtplugError::ButtplugHandshakeError(_s) => _s.message,
            ButtplugError::ButtplugUnknownError(_s) => _s.message,
        };
        Error::new(code, &msg)
    }
}

#[cfg(feature = "serialize_json")]
#[cfg(test)]
mod test {
    use crate::core::messages::{ButtplugMessageUnion, Error, ErrorCode};

    const ERROR_STR: &str =
        "{\"Error\":{\"Id\":0,\"ErrorCode\":1,\"ErrorMessage\":\"Test Error\"}}";

    #[test]
    fn test_error_serialize() {
        let error =
            ButtplugMessageUnion::Error(Error::new(ErrorCode::ErrorHandshake, "Test Error"));
        let js = serde_json::to_string(&error).unwrap();
        assert_eq!(ERROR_STR, js);
    }

    #[test]
    fn test_error_deserialize() {
        let union: ButtplugMessageUnion = serde_json::from_str(&ERROR_STR).unwrap();
        assert_eq!(
            ButtplugMessageUnion::Error(Error::new(ErrorCode::ErrorHandshake, "Test Error")),
            union
        );
    }
}