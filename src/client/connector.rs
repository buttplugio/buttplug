use std::error::Error;
use std::fmt;
use async_trait::async_trait;
use super::client::ButtplugClientError;
use crate::core::messages::ButtplugMessageUnion;
use crate::server::server::ButtplugServer;

#[derive(Debug)]
pub struct ButtplugClientConnectorError {
    pub message: String,
}

impl fmt::Display for ButtplugClientConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Init Error: {}", self.message)
    }
}

impl Error for ButtplugClientConnectorError {
    fn description(&self) -> &str {
        self.message.as_str()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[async_trait]
pub trait ButtplugClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError>;
    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError>;
    async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError>;
}

pub struct ButtplugEmbeddedClientConnector {
    server: ButtplugServer,
    server_name: String,
    max_ping_time: u32
}

impl ButtplugEmbeddedClientConnector {
    pub fn new(name: &str, max_ping_time: u32) -> ButtplugEmbeddedClientConnector {
        ButtplugEmbeddedClientConnector {
            server: ButtplugServer::new(&name, max_ping_time),
            server_name: name.to_string(),
            max_ping_time: max_ping_time
        }
    }
}

#[async_trait]
impl ButtplugClientConnector for ButtplugEmbeddedClientConnector {
    async fn connect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    fn disconnect(&mut self) -> Option<ButtplugClientConnectorError> {
        None
    }

    async fn send(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        self.server
            .send_message(msg)
            .await
            .map_err(|x| ButtplugClientError::ButtplugError(x))
    }
}

// The embedded connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.
