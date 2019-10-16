pub mod connector;
pub mod device;
mod messagesorter;

use std::error::Error;
use std::fmt;
use pharos::{Pharos, Observable, Events, ObserveConfig, Channel};
use crate::core::messages::{RequestServerInfo,
                            ButtplugMessage,
                            ButtplugMessageUnion,
                            StartScanning};
use connector::{ButtplugClientConnector,
                       ButtplugClientConnectorError};
use crate::core::errors::{ButtplugError,
                          ButtplugMessageError,
                          ButtplugInitError};
use device::{ButtplugClientDevice, ButtplugClientDeviceMessage};
use futures_channel::mpsc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtplugClientEvent {
    ScanningFinished,
    DeviceAdded,
    DeviceRemoved,
    Log,
    PingTimeout,
    ServerDisconnect,
}

#[derive(Debug, Clone)]
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

pub struct ButtplugClient<'a> {
    pub client_name: String,
    pub server_name: Option<String>,
    pub max_ping_time: u32,
    pub observers: Pharos<ButtplugClientEvent>,
    devices: Vec<ButtplugClientDevice<'a>>,
    device_receivers: Vec<mpsc::UnboundedReceiver<ButtplugClientDeviceMessage<'a>>>,
    connector: Option<Box<dyn ButtplugClientConnector>>,
}

unsafe impl<'a> Sync for ButtplugClient<'a> {}
unsafe impl<'a> Send for ButtplugClient<'a> {}

impl<'a> ButtplugClient<'a> {
    pub fn new(name: &str) -> ButtplugClient<'a> {
        ButtplugClient {
            client_name: name.to_string(),
            max_ping_time: 0,
            server_name: None,
            connector: None,
            observers: Pharos::default(),
            devices: vec!(),
            device_receivers: vec!(),
        }
    }

    pub async fn connect(&mut self, mut connector: impl ButtplugClientConnector + 'static) -> Result<(), ButtplugClientError> {
        if self.connector.is_some() {
            return Result::Err(ButtplugClientError::ButtplugClientConnectorError(ButtplugClientConnectorError { message: "Client already connected".to_string() }));
        }

        let mut recv = connector.get_event_receiver();

        match connector.connect().await {
            Some (_s) => return Result::Err(ButtplugClientError::ButtplugClientConnectorError(_s)),
            None => {
                println!("Init in connect");
                self.connector = Option::Some(Box::new(connector));
                match self.init().await {
                    Ok(_) => {
                        Ok(())
                    }
                    Err(x) => {
                        self.connector = None;
                        Err(x)
                    }
                }
            }
        }
    }

    async fn init(&mut self) -> Result<(), ButtplugClientError> {
        println!("Initing");
        self.send_message(&RequestServerInfo::new(&self.client_name, 1).as_union())
            .await
            .map_err(|x| x)
            .and_then(|x| {
                // TODO Error message case may need to be implemented here when
                // we aren't only using embedded connectors.
                if let ButtplugMessageUnion::ServerInfo(server_info) = x {
                    self.server_name = Option::Some(server_info.server_name);
                    self.max_ping_time = server_info.max_ping_time;
                    Ok(())
                } else {
                    Err(ButtplugClientError::ButtplugError(ButtplugError::ButtplugInitError(ButtplugInitError { message: "Did not receive expected ServerInfo or Error messages.".to_string() })))
                }
            })
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

    pub async fn start_scanning(&mut self) -> Result<(), ButtplugClientError> {
        self
            .send_message_expect_ok(&ButtplugMessageUnion::StartScanning(StartScanning::new()))
            .await
    }

    async fn send_message(&mut self, msg: &ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        if let Some(ref mut connector) = self.connector {
            connector
                .send(msg)
                .await
                .map_err(|x| x)
        } else {
            Err(ButtplugClientError::ButtplugClientConnectorError(ButtplugClientConnectorError { message: "Client not Connected.".to_string() }))
        }
    }

    // TODO This should return Option<ButtplugClientError> but there's a known size issue.
    async fn send_message_expect_ok(&mut self, msg: &ButtplugMessageUnion) -> Result<(), ButtplugClientError> {
        self.send_message(msg)
            .await
            .and_then(|x: ButtplugMessageUnion| {
                match x {
                    ButtplugMessageUnion::Ok(_) => Ok(()),
                    _ => Err(ButtplugClientError::ButtplugError(ButtplugError::ButtplugMessageError(ButtplugMessageError { message: "Got non-Ok message back".to_string() })))
                }
            })
    }

    pub async fn on_message_received(&mut self, msg: &ButtplugMessageUnion) {
        match msg {
            ButtplugMessageUnion::ScanningFinished(_) => {},
            ButtplugMessageUnion::DeviceList(_msg) => {
                for info in _msg.devices.iter() {
                    let (send, recv) = mpsc::unbounded::<ButtplugClientDeviceMessage>();
                    self.device_receivers.push(recv);
                    self.devices.push(ButtplugClientDevice::from((&info.clone(), send)));
                    // TODO Fire DeviceAdded here, once we figure out how events work

                    // TODO Actually put the receiver into a loop somewhere. Maybe the connector?
                }
            },
            ButtplugMessageUnion::DeviceAdded(_msg) => {
                let (send, recv) = mpsc::unbounded::<ButtplugClientDeviceMessage>();
                self.device_receivers.push(recv);
                self.devices.push(ButtplugClientDevice::from((_msg, send)));
                // TODO Fire DeviceAdded here, once we figure out how events work

                // TODO Actually put the receiver into a loop somewhere. Maybe the connector?
            },
            ButtplugMessageUnion::DeviceRemoved(_) => {},
            //ButtplugMessageUnion::Log(_) => {}
            _ => panic!("Unhandled incoming message!")
        }
    }

    pub fn get_default_observer(&mut self) -> Result< Events<ButtplugClientEvent>, pharos::Error > {
        Ok(self.observe(Channel::Unbounded.into()).expect( "observe" ))
    }
}

impl<'a> Observable<ButtplugClientEvent> for ButtplugClient<'a> {
   type Error = pharos::Error;

   fn observe(&mut self, options: ObserveConfig<ButtplugClientEvent>) -> Result< Events<ButtplugClientEvent>, Self::Error >
   {
       self.observers.observe(options)
   }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::client::connector::ButtplugEmbeddedClientConnector;
    use async_std::task;

    async fn connect_test_client() -> ButtplugClient<'static> {
        let mut client = ButtplugClient::new("Test Client");
        assert!(client.connect(ButtplugEmbeddedClientConnector::new("Test Server", 0)).await.is_ok());
        assert!(client.connected());
        client
    }

    #[test]
    fn test_connect_status() {
        task::block_on(async {
            connect_test_client().await;
        });
    }

    #[test]
    fn test_disconnect_status() {
        task::block_on(async {
            let mut client = connect_test_client().await;
            assert!(client.disconnect().is_ok());
            assert!(!client.connected());
        });
    }

    #[test]
    fn test_disconnect_with_no_connect() {
        let mut client = ButtplugClient::new("Test Client");
        assert!(client.disconnect().is_err());
    }

    #[test]
    fn test_connect_init() {
        task::block_on(async {
            let client = connect_test_client().await;
            assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
        });
    }

    #[test]
    fn test_start_scanning() {
        task::block_on(async {
            let mut client = connect_test_client().await;
            assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
            assert!(client.start_scanning().await.is_ok());
        });
    }

    #[test]
    fn test_scanning_finished() {
        task::block_on(async {
            let mut client = connect_test_client().await;
            assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
            assert!(client.start_scanning().await.is_ok());
        });
    }

    // Failure on server version error is unit tested in server.
}
