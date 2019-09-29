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
    pub client_name: String,
    pub server_name: Option<String>,
    connector: Option<Box<dyn ButtplugClientConnector>>
}

impl ButtplugClient {
    pub fn new(name: &str) -> ButtplugClient {
        ButtplugClient {
            client_name: name.to_string(),
            server_name: None,
            connector: None
        }
    }

    pub fn connect<T: ButtplugClientConnector + 'static>(&mut self, mut connector: T) -> Result<(), ButtplugClientError> {
        match connector.connect() {
            Some (_s) => return Result::Err(ButtplugClientError::ButtplugClientConnectorError(_s)),
            None => self.connector = Option::Some(Box::new(connector)),
        }
        Result::Ok(())
    }

    pub fn connected(&self) -> bool {
        return self.connector.is_some();
    }

    pub fn disconnect(&mut self) -> Result<(), ButtplugClientError> {
        if self.connector.is_none() {
            return Result::Err(ButtplugClientError::ButtplugClientConnectorError(ButtplugClientConnectorError { message: "Client not connected".to_string() }));
        }
        let mut connector = self.connector.take().unwrap();
        connector.disconnect();
        Result::Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::client::connector::ButtplugEmbeddedClientConnector;

    #[test]
    fn test_embedded_connector_connect() {
        let mut client = ButtplugClient::new("Test Client");
        client.connect(ButtplugEmbeddedClientConnector::new("Test Server", 0));
        assert!(client.connected());
    }

    #[test]
    fn test_embedded_connector_disconnect() {
        let mut client = ButtplugClient::new("Test Client");
        client.connect(ButtplugEmbeddedClientConnector::new("Test Server", 0));
        assert!(client.disconnect().is_ok());
        assert!(!client.connected());
    }

    #[test]
    fn test_embedded_connector_disconnect_with_no_connect() {
        let mut client = ButtplugClient::new("Test Client");
        assert!(client.disconnect().is_err());
    }
}
