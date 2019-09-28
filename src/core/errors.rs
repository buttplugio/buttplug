use std::fmt;
use std::error::Error;
use super::messages::ButtplugMessageUnion;
use super::messages;

pub trait ButtplugErrorTrait {
    fn as_message(&self) -> ButtplugMessageUnion;
}

#[derive(Debug)]
pub struct ButtplugInitError {
    pub message: String,
}

impl ButtplugErrorTrait for ButtplugInitError {
    fn as_message(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Error(messages::Error {
            id: 0,
            error_code: messages::ErrorCode::ErrorInit,
            error_message: self.message.clone(),
        })
    }
}

impl fmt::Display for ButtplugInitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Init Error: {}", self.message)
    }
}

impl Error for ButtplugInitError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug)]
pub struct ButtplugMessageError {
    pub message: String,
}

impl ButtplugErrorTrait for ButtplugMessageError {
    fn as_message(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Error(messages::Error {
            id: 0,
            error_code: messages::ErrorCode::ErrorMessage,
            error_message: self.message.clone(),
        })
    }
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

#[derive(Debug)]
pub struct ButtplugPingError {
    pub message: String,
}

impl ButtplugErrorTrait for ButtplugPingError {
    fn as_message(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Error(messages::Error {
            id: 0,
            error_code: messages::ErrorCode::ErrorPing,
            error_message: self.message.clone(),
        })
    }
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

#[derive(Debug)]
pub struct ButtplugDeviceError {
    pub message: String,
}

impl ButtplugErrorTrait for ButtplugDeviceError {
    fn as_message(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Error(messages::Error {
            id: 0,
            error_code: messages::ErrorCode::ErrorDevice,
            error_message: self.message.clone(),
        })
    }
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

#[derive(Debug)]
pub struct ButtplugUnknownError {
    pub message: String,
}

impl ButtplugErrorTrait for ButtplugUnknownError {
    fn as_message(&self) -> ButtplugMessageUnion {
        ButtplugMessageUnion::Error(messages::Error {
            id: 0,
            error_code: messages::ErrorCode::ErrorUnknown,
            error_message: self.message.clone(),
        })
    }
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

#[derive(Debug)]
pub enum ButtplugError {
    ButtplugInitError(ButtplugInitError),
    ButtplugMessageError(ButtplugMessageError),
    ButtplugPingError(ButtplugPingError),
    ButtplugDeviceError(ButtplugDeviceError),
    ButtplugUnknownError(ButtplugUnknownError),
}

impl ButtplugErrorTrait for ButtplugError {
    fn as_message(&self) -> ButtplugMessageUnion {
        match *self {
            ButtplugError::ButtplugDeviceError(ref e) => e.as_message(),
            ButtplugError::ButtplugMessageError(ref e) => e.as_message(),
            ButtplugError::ButtplugPingError(ref e) => e.as_message(),
            ButtplugError::ButtplugInitError(ref e) => e.as_message(),
            ButtplugError::ButtplugUnknownError(ref e) => e.as_message(),
        }
    }
}

impl fmt::Display for ButtplugError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ButtplugError::ButtplugDeviceError(ref e) => e.fmt(f),
            ButtplugError::ButtplugMessageError(ref e) => e.fmt(f),
            ButtplugError::ButtplugPingError(ref e) => e.fmt(f),
            ButtplugError::ButtplugInitError(ref e) => e.fmt(f),
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
            ButtplugError::ButtplugInitError(ref e) => e.description(),
            ButtplugError::ButtplugUnknownError(ref e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
