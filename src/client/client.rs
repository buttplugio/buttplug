use std::error::Error;
use std::fmt;
use super::connector::ButtplugClientConnector;
use super::connector::ButtplugClientConnectorError;
use crate::core::errors::ButtplugError;

#[derive(Debug)]
pub enum ButtplugClientError {
    ButtplugClientConnectorError(ButtplugClientConnectorError),
    ButtplugError(ButtplugError),
}

impl fmt::Display for ButtplugClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ButtplugClientError::ButtplugError(ref e) => e.fmt(f),
            ButtplugClientError::ButtplugClientConnectorError(ref e) => e.fmt(f),
        }
    }
}

impl Error for ButtplugClientError {
    fn description(&self) -> &str {
        match *self {
            ButtplugClientError::ButtplugError(ref e) => e.description(),
            ButtplugClientError::ButtplugClientConnectorError(ref e) => e.description(),
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


pub struct ButtplugClient {
    pub name: String,
    connector: Option<Box<dyn ButtplugClientConnector>>
}

impl ButtplugClient {
    pub fn new(name: &str) -> ButtplugClient {
        ButtplugClient {
            name: name.to_string(),
            connector: None
        }
    }

    pub fn connect<T: ButtplugClientConnector + 'static>(&mut self, connector: T) where {
        self.connector = Option::Some(Box::new(connector));
    }

    pub fn connected(&self) -> bool {
        return self.connector.is_some();
    }
}

#[cfg(test)]
mod test {
    use super::*;

}
