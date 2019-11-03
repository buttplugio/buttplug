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

/// Handshake errors occur while a client is connecting to a server. This
/// usually involves protocol handshake errors. For connector errors (i.e. when
/// a remote network connection cannot be established), see
/// [crate::client::connector::ButtplugClientConnectorError].
#[derive(Debug, Clone)]
pub struct ButtplugHandshakeError {
    /// Message for the handshake error.
    pub message: String,
}

impl fmt::Display for ButtplugHandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Init Error: {}", self.message)
    }
}

impl Error for ButtplugHandshakeError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Message errors occur when a message is somehow malformed on creation, or
/// received unexpectedly by a client or server.
#[derive(Debug, Clone)]
pub struct ButtplugMessageError {
    pub message: String,
}

impl fmt::Display for ButtplugMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Message Error: {}", self.message)
    }
}

impl Error for ButtplugMessageError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Ping errors occur when a server requires a ping response (set up during
/// connection handshake), and the client does not return a response in the
/// alloted timeframe. This also signifies a server disconnect.
#[derive(Debug, Clone)]
pub struct ButtplugPingError {
    pub message: String,
}

impl fmt::Display for ButtplugPingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ping Error: {}", self.message)
    }
}

impl Error for ButtplugPingError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Device errors occur during device interactions, including sending
/// unsupported message commands, addressing the wrong number of device
/// attributes, etc...
#[derive(Debug, Clone)]
pub struct ButtplugDeviceError {
    pub message: String,
}

impl fmt::Display for ButtplugDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Device Error: {}", self.message)
    }
}

impl Error for ButtplugDeviceError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// Unknown errors occur in exceptional circumstances where no other error type
/// will suffice. These are rare and usually fatal (disconnecting) errors.
#[derive(Debug, Clone)]
pub struct ButtplugUnknownError {
    pub message: String,
}

impl fmt::Display for ButtplugUnknownError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unknown Error: {}", self.message)
    }
}

impl Error for ButtplugUnknownError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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
    fn description(&self) -> &str {
        match *self {
            ButtplugError::ButtplugDeviceError(ref e) => e.description(),
            ButtplugError::ButtplugMessageError(ref e) => e.description(),
            ButtplugError::ButtplugPingError(ref e) => e.description(),
            ButtplugError::ButtplugHandshakeError(ref e) => e.description(),
            ButtplugError::ButtplugUnknownError(ref e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
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
