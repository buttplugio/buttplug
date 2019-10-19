use super::messages::{self, ErrorCode};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ButtplugInitError {
    pub message: String,
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

#[derive(Debug, Clone)]
pub enum ButtplugError {
    ButtplugInitError(ButtplugInitError),
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

impl From<messages::Error> for ButtplugError {
    fn from(error: messages::Error) -> Self {
        match error.error_code {
            ErrorCode::ErrorDevice => ButtplugError::ButtplugDeviceError(ButtplugDeviceError {
                message: error.error_message,
            }),
            ErrorCode::ErrorMessage => ButtplugError::ButtplugMessageError(ButtplugMessageError {
                message: error.error_message,
            }),
            ErrorCode::ErrorInit => ButtplugError::ButtplugInitError(ButtplugInitError {
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
