pub mod connector;
pub mod device;
pub mod internal;
mod messagesorter;

use crate::core::errors::{ButtplugError, ButtplugInitError, ButtplugMessageError};
use crate::core::messages::{
    ButtplugMessage, ButtplugMessageUnion, RequestServerInfo, StartScanning,
};
use connector::{ButtplugClientConnector, ButtplugClientConnectorError};
use device::ButtplugClientDevice;
use futures::{Future, SinkExt, StreamExt};
use futures_channel::mpsc;
use internal::{
    ButtplugClientInternalLoop, ButtplugClientMessageFuture, ButtplugInternalClientMessage,
};
use pharos::{Channel, Events, Observable, ObserveConfig, Pharos};
use std::error::Error;
use std::fmt;

#[derive(Clone)]
pub enum ButtplugClientEvent {
    ScanningFinished,
    DeviceAdded(ButtplugClientDevice),
    DeviceRemoved(ButtplugClientDevice),
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

pub struct ButtplugClient {
    pub client_name: String,
    pub server_name: Option<String>,
    pub observers: Pharos<ButtplugClientEvent>,
    devices: Vec<ButtplugClientDevice>,
    message_sender: mpsc::UnboundedSender<ButtplugInternalClientMessage>,
    event_receiver: mpsc::UnboundedReceiver<ButtplugMessageUnion>,
    connected: bool,
}

// impl Clone for ButtplugClient {
//     fn clone(&mut self) -> Self {
//     }
// }

unsafe impl Sync for ButtplugClient {}
unsafe impl Send for ButtplugClient {}

impl ButtplugClient {
    pub fn new(name: &str) -> (ButtplugClient, impl Future) {
        let (event_sender, event_receiver) = mpsc::unbounded();
        let mut internal_loop = ButtplugClientInternalLoop::new(event_sender);
        (
            ButtplugClient {
                client_name: name.to_string(),
                server_name: None,
                observers: Pharos::default(),
                devices: vec![],
                event_receiver,
                message_sender: internal_loop.get_client_sender(),
                connected: false,
            },
            async move {
                loop {
                    internal_loop.wait_for_event().await;
                }
            },
        )
    }

    pub async fn connect(
        &mut self,
        mut connector: impl ButtplugClientConnector + 'static,
    ) -> Result<(), ButtplugClientError> {
        let fut = ButtplugClientMessageFuture::default();
        self.message_sender
            .send(ButtplugInternalClientMessage::Connect(
                Box::new(connector),
                fut.get_state_ref().clone(),
            ))
            .await;
        println!("Waiting on connect");
        let msg = fut.await;
        println!("connected in client");
        self.connected = true;
        println!("calling init");
        self.init().await.unwrap();
        Ok(())
    }

    async fn init(&mut self) -> Result<(), ButtplugClientError> {
        println!("Initing");
        let res = self
            .send_message(&RequestServerInfo::new(&self.client_name, 1).as_union())
            .await;
        match res {
            Ok(msg) => {
                println!("got message back");
                // TODO Error message case may need to be implemented here when
                // we aren't only using embedded connectors.
                if let ButtplugMessageUnion::ServerInfo(server_info) = msg {
                    self.server_name = Option::Some(server_info.server_name);
                    // TODO Handle ping time in the internal event loop
                    Ok(())
                } else {
                    Err(ButtplugClientError::ButtplugError(
                        ButtplugError::ButtplugInitError(ButtplugInitError {
                            message: "Did not receive expected ServerInfo or Error messages."
                                .to_string(),
                        }),
                    ))
                }
            }
            Err(_) => Ok(()),
        }
    }

    pub fn connected(&self) -> bool {
        return self.connected;
    }

    pub fn disconnect(&mut self) -> Result<(), ButtplugClientError> {
        // if self.connector.is_none() {
        //     return Result::Err(ButtplugClientError::ButtplugClientConnectorError(
        //         ButtplugClientConnectorError {
        //             message: "Client not connected".to_string(),
        //         },
        //     ));
        // }
        // let mut connector = self.connector.take().unwrap();
        // connector.disconnect();
        self.connected = false;
        Result::Ok(())
    }

    pub async fn start_scanning(&mut self) -> Result<(), ButtplugClientError> {
        self.send_message_expect_ok(&ButtplugMessageUnion::StartScanning(StartScanning::new()))
            .await
    }

    async fn send_message(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        if self.connected {
            let fut = ButtplugClientMessageFuture::default();
            self.message_sender
                .send(ButtplugInternalClientMessage::Message((
                    msg.clone(),
                    fut.get_state_ref().clone(),
                )))
                .await;
            Ok(fut.await)
        } else {
            Err(ButtplugClientError::ButtplugClientConnectorError(
                ButtplugClientConnectorError {
                    message: "Client not Connected.".to_string(),
                },
            ))
        }
    }

    // TODO This should return Option<ButtplugClientError> but there's a known size issue.
    async fn send_message_expect_ok(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<(), ButtplugClientError> {
        let msg = self.send_message(msg).await;
        match msg.unwrap() {
            ButtplugMessageUnion::Ok(_) => Ok(()),
            _ => Err(ButtplugClientError::ButtplugError(
                ButtplugError::ButtplugMessageError(ButtplugMessageError {
                    message: "Got non-Ok message back".to_string(),
                }),
            )),
        }
    }

    pub async fn wait_for_event(&mut self) {
        match self.event_receiver.next().await.unwrap() {
            ButtplugMessageUnion::ScanningFinished(_) => {}
            ButtplugMessageUnion::DeviceList(_msg) => {
                for info in _msg.devices.iter() {
                    let device =
                        ButtplugClientDevice::from((&info.clone(), self.message_sender.clone()));
                    self.devices.push(device.clone());
                    self.observers
                        .send(ButtplugClientEvent::DeviceAdded(device))
                        .await
                        .expect("Events should be able to send");
                }
            }
            ButtplugMessageUnion::DeviceAdded(_msg) => {
                println!("Got a device added message!");
                let device = ButtplugClientDevice::from((&_msg, self.message_sender.clone()));
                self.devices.push(device.clone());
                println!("Sending to observers!");
                self.observers
                    .send(ButtplugClientEvent::DeviceAdded(device))
                    .await
                    .expect("Events should be able to send");
                println!("Observers sent!");
            }
            ButtplugMessageUnion::DeviceRemoved(_) => {}
            //ButtplugMessageUnion::Log(_) => {}
            _ => panic!("Unhandled incoming message!"),
        }
    }

    pub fn get_default_observer(&mut self) -> Result<Events<ButtplugClientEvent>, pharos::Error> {
        Ok(self.observe(Channel::Unbounded.into()).expect("observe"))
    }
}

impl Observable<ButtplugClientEvent> for ButtplugClient {
    type Error = pharos::Error;

    fn observe(
        &mut self,
        options: ObserveConfig<ButtplugClientEvent>,
    ) -> Result<Events<ButtplugClientEvent>, Self::Error> {
        self.observers.observe(options)
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::client::connector::ButtplugEmbeddedClientConnector;
    use async_std::task;

    async fn connect_test_client() -> ButtplugClient {
        let (mut client, fut_loop) = ButtplugClient::new("Test Client");
        task::spawn(async move {
            fut_loop.await;
        });
        assert!(client
            .connect(ButtplugEmbeddedClientConnector::new("Test Server", 0))
            .await
            .is_ok());
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

    // #[test]
    // fn test_disconnect_with_no_connect() {
    //     let mut client = ButtplugClient::new("Test Client");
    //     assert!(client.disconnect().is_err());
    // }

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
