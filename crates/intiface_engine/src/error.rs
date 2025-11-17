use buttplug_core::errors::ButtplugError;
use buttplug_server::ButtplugServerError;
use std::{error::Error, fmt};

#[derive(Debug)]
pub struct IntifaceError {
  reason: String,
}

impl IntifaceError {
  pub fn new(error_msg: &str) -> Self {
    Self {
      reason: error_msg.to_owned(),
    }
  }
}

impl fmt::Display for IntifaceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.reason)
  }
}

impl Error for IntifaceError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

#[derive(Debug)]
pub enum IntifaceEngineError {
  IoError(std::io::Error),
  ButtplugServerError(ButtplugServerError),
  ButtplugError(ButtplugError),
  IntifaceError(IntifaceError),
}

impl From<std::io::Error> for IntifaceEngineError {
  fn from(err: std::io::Error) -> Self {
    IntifaceEngineError::IoError(err)
  }
}

impl From<ButtplugError> for IntifaceEngineError {
  fn from(err: ButtplugError) -> Self {
    IntifaceEngineError::ButtplugError(err)
  }
}

impl From<IntifaceError> for IntifaceEngineError {
  fn from(err: IntifaceError) -> Self {
    IntifaceEngineError::IntifaceError(err)
  }
}
