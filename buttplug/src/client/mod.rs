// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub mod connector;
pub mod device;
pub mod internal;
mod messagesorter;

use connector::{ButtplugClientConnector, ButtplugClientConnectorError};
use device::ButtplugClientDevice;
use internal::{
    ButtplugClientInternalLoop, ButtplugClientMessageFuture, ButtplugInternalClientMessage,
};

use crate::core::{
    errors::{ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
    messages::{
        ButtplugMessage, ButtplugMessageUnion, RequestServerInfo, StartScanning,
    }
};

use futures::{Future, StreamExt};
use async_std::{sync::{channel, Sender, Receiver}};
use std::error::Error;
use std::fmt;

/// Enum representing different events that can be emitted by a client.
///
/// These events are created by the server and sent to the client, and represent
/// unrequested actions that the client will need to respond to, or that
/// applications using the client may be interested in.
#[derive(Clone)]
pub enum ButtplugClientEvent {
    /// Emitted when a scanning session (started via a StartScanning call on
    /// [ButtplugClient]) has finished.
    ScanningFinished,
    /// Emitted when a device has been added to the server. Includes a
    /// [ButtplugClientDevice] object representing the device.
    DeviceAdded(ButtplugClientDevice),
    /// Emitted when a device has been removed from the server. Includes a
    /// [ButtplugClientDevice] object representing the device.
    DeviceRemoved(ButtplugClientDevice),
    /// Emitted when log messages are sent from the server.
    // TODO This needs an actual type sent along with it.
    Log,
    /// Emitted when a client has not pinged the server in a sufficient amount
    /// of time.
    PingTimeout,
    /// Emitted when a client connector detects that the server has
    /// disconnected.
    ServerDisconnect,
}

/// Represents all of the different types of errors a ButtplugClient can return.
///
/// Clients can return two types of errors:
///
/// - [ButtplugClientConnectorError], which means there was a problem with the
/// connection between the client and the server, like a network connection
/// issue.
/// - [ButtplugError], which is an error specific to the Buttplug Protocol.
#[derive(Debug, Clone)]
pub enum ButtplugClientError {
    /// Connector error
    ButtplugClientConnectorError(ButtplugClientConnectorError),
    /// Protocol error
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

/// Struct used by applications to communicate with a Buttplug Server.
///
/// Buttplug Clients provide an API layer on top of the Buttplug Protocol that
/// handles boring things like message creation and pairing, protocol ordering,
/// etc... This allows developers to concentrate on controlling hardware with
/// the API.
///
/// Clients serve a few different purposes:
/// - Managing connections to servers, thru [ButtplugClientConnector]s
/// - Emitting events received from the Server
/// - Holding state related to the server (i.e. what devices are currently
///   connected, etc...)
#[derive(Clone)]
pub struct ButtplugClient {
    /// The client name. Depending on the connection type and server being used,
    /// this name is sometimes shown on the server logs or GUI.
    pub client_name: String,
    /// The server name. Once connected, this contains the name of the server,
    /// so we can know what we're connected to.
    pub server_name: Option<String>,
    // A vector of devices currently connected to the server, as represented by
    // [ButtplugClientDevice] types.
    devices: Vec<ButtplugClientDevice>,
    message_sender: Sender<ButtplugInternalClientMessage>,
    // Receives event notifications from the ButtplugInternalClientLoop
    event_receiver: Receiver<ButtplugMessageUnion>,
    // True if the connector is currently connected, and handshake was
    // successful.
    connected: bool,
}

unsafe impl Sync for ButtplugClient {}
unsafe impl Send for ButtplugClient {}

impl ButtplugClient {
    pub fn new(name: &str) -> (ButtplugClient, impl Future) {
        let (event_sender, event_receiver) = channel(256);
        let mut internal_loop = ButtplugClientInternalLoop::new(event_sender);
        (
            ButtplugClient {
                client_name: name.to_string(),
                server_name: None,
                devices: vec![],
                event_receiver,
                message_sender: internal_loop.get_client_sender(),
                connected: false,
            },
            async move {
                loop {
                    // TODO Loop this in wait_for_event, not outside of it.
                    internal_loop.wait_for_event().await;
                }
            },
        )
    }

    pub async fn connect(
        &mut self,
        connector: impl ButtplugClientConnector + 'static,
    ) -> Option<ButtplugClientError> {
        let fut = ButtplugClientMessageFuture::default();
        self.message_sender
            .send(ButtplugInternalClientMessage::Connect(
                Box::new(connector),
                fut.get_state_ref().clone(),
            ))
            .await;
        info!("Waiting on connect");
        let msg = fut.await;
        info!("connected in client");
        self.connected = true;
        info!("calling init");
        self.init().await.unwrap();
        None
    }

    async fn init(&mut self) -> Option<ButtplugClientError> {
        info!("Initing");
        let res = self
            .send_message(&RequestServerInfo::new(&self.client_name, 1).as_union())
            .await;
        match res {
            Ok(msg) => {
                info!("got message back");
                // TODO Error message case may need to be implemented here when
                // we aren't only using embedded connectors.
                if let ButtplugMessageUnion::ServerInfo(server_info) = msg {
                    self.server_name = Option::Some(server_info.server_name);
                    // TODO Handle ping time in the internal event loop
                    None
                } else {
                    Some(ButtplugClientError::ButtplugError(
                        ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError {
                            message: "Did not receive expected ServerInfo or Error messages."
                                .to_string(),
                        }),
                    ))
                }
            }
            Err(_) => None,
        }
    }

    pub fn connected(&self) -> bool {
        return self.connected;
    }

    pub fn disconnect(&mut self) -> Option<ButtplugClientError> {
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
        None
    }

    pub async fn start_scanning(&mut self) -> Option<ButtplugClientError> {
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

    pub async fn wait_for_event(&mut self) -> Vec<ButtplugClientEvent> {
        let mut events = vec!();
        match self.event_receiver.next().await.unwrap() {
            ButtplugMessageUnion::ScanningFinished(_) => {}
            ButtplugMessageUnion::DeviceList(_msg) => {
                for info in _msg.devices.iter() {
                    let device =
                        ButtplugClientDevice::from((&info.clone(), self.message_sender.clone()));
                    self.devices.push(device.clone());
                    events.push(ButtplugClientEvent::DeviceAdded(device));
                }
            }
            ButtplugMessageUnion::DeviceAdded(_msg) => {
                info!("Got a device added message!");
                let device = ButtplugClientDevice::from((&_msg, self.message_sender.clone()));
                self.devices.push(device.clone());
                info!("Sending to observers!");
                events.push(ButtplugClientEvent::DeviceAdded(device));
                info!("Observers sent!");
            }
            ButtplugMessageUnion::DeviceRemoved(_) => {}
            //ButtplugMessageUnion::Log(_) => {}
            _ => panic!("Unhandled incoming message!"),
        }
        events
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::client::connector::ButtplugEmbeddedClientConnector;
    use async_std::task;
    use env_logger;

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
