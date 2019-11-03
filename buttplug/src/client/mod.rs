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
///
/// When a client is first created, it will be able to create an internal loop
/// as a Future, and return it via the [ButtplugClient::get_loop] call. This
/// loop needs to be awaited before awaiting other client calls (like
/// [ButtplugClient::connect]), otherwise the system will panic.
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
    // Sender to relay messages to the internal client loop
    message_sender: Option<Sender<ButtplugInternalClientMessage>>,
    // Receives event notifications from the ButtplugInternalClientLoop
    event_receiver: Option<Receiver<ButtplugMessageUnion>>,
    // True if the connector is currently connected, and handshake was
    // successful.
    connected: bool,
}

unsafe impl Sync for ButtplugClient {}
unsafe impl Send for ButtplugClient {}

impl ButtplugClient {
    /// Creates a new ButtplugClient.
    ///
    /// # Parameters
    ///
    /// - `name`: Name to be given to the client (see [ButtplugClient::client_name]).
    pub fn new(name: &str) -> ButtplugClient {
        ButtplugClient {
            client_name: name.to_string(),
            server_name: None,
            devices: vec![],
            event_receiver: None,
            message_sender: None,
            connected: false,
        }
    }

    /// Returns a future representing the internal loop for the client.
    ///
    /// The internal loop will need to be run alongside a future where client
    /// logic is performed.
    pub fn get_loop(&mut self) -> impl Future {
        let (event_sender, event_receiver) = channel(256);
        let (message_sender, message_receiver) = channel(256);
        let mut internal_loop = ButtplugClientInternalLoop::new(event_sender, message_receiver);
        self.event_receiver = Some(event_receiver);
        self.message_sender = Some(message_sender);
        async move {
            loop {
                // TODO Loop this in wait_for_event, not outside of it.
                internal_loop.wait_for_event().await;
            }
        }
    }

    /// Connects and runs handshake flow with
    /// [crate::server::server::ButtplugServer], either local or remote.
    ///
    /// Tries to connect to a server via the given [ButtplugClientConnector]
    /// struct. If connection is successful, also runs the handshake flow and
    /// retrieves a list of currently connected devices. These devices will be
    /// emitted as [ButtplugClientEvent::DeviceAdded] events next time
    /// [ButtplugClient::wait_for_event] is run.
    ///
    /// # Parameters
    ///
    /// - `connector`: A connector of some type that will handle the connection
    /// to the server. The core library ships with an "embedded" connector
    /// ([connector::ButtplugEmbeddedClientConnector]) that will run a server
    /// in-process with the client, or there are add-on libraries like
    /// buttplug-ws-connector that will handle other communication methods like
    /// websockets, TCP/UDP, etc...
    ///
    /// # Returns
    ///
    /// An `Option` which is:
    ///
    /// - None if connection succeeded
    /// - Some containing a [ButtplugClientError] on connection failure.
    pub async fn connect(
        &mut self,
        connector: impl ButtplugClientConnector + 'static,
    ) -> Option<ButtplugClientError> {
        debug!("Running client connection.");

        // Send the connector to the internal loop for management. Once we throw
        // the connector over, the internal loop will handle connecting and any
        // further communications with the server, if connection is successful.
        let fut = ButtplugClientMessageFuture::default();
        let msg = ButtplugInternalClientMessage::Connect(
            Box::new(connector),
            fut.get_state_clone(),
        );
        let err = self.send_internal_message(msg).await;
        // If we get back an error, we couldn't connect, so exit early.
        if err.is_some() {
            return err;
        }

        debug!("Waiting on internal loop to connect");
        fut.await;

        info!("Client connected to server, running handshake.");
        // Set connected to true, since running the handshake requires the
        // ability to send messages.
        self.connected = true;
        self.handshake().await
    }

    // Runs the handshake flow with the server.
    //
    // Sends over RequestServerInfo, gets back ServerInfo, sets up ping timer if
    // needed.
    async fn handshake(&mut self) -> Option<ButtplugClientError> {
        info!("Running handshake with server.");
        let res = self
            .send_message(&RequestServerInfo::new(&self.client_name, 1).as_union())
            .await;
        match res {
            Ok(msg) => {
                debug!("Got ServerInfo return.");
                if let ButtplugMessageUnion::ServerInfo(server_info) = msg {
                    info!("Connected to {}", server_info.server_name);
                    self.server_name = Option::Some(server_info.server_name);
                    // TODO Handle ping time in the internal event loop
                    None
                } else {
                    // TODO Should disconnect here.
                    Some(ButtplugClientError::ButtplugError(
                        ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError {
                            message: "Did not receive expected ServerInfo or Error messages."
                                .to_string(),
                        }),
                    ))
                }
            }
            // TODO Error message case may need to be implemented here when
            // we aren't only using embedded connectors.
            Err(_) => None,
        }
    }

    /// Returns true if client is currently connected to server.
    pub fn connected(&self) -> bool {
        return self.connected;
    }

    /// Disconnects from server, if connected.
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

    /// Tells server to start scanning for devices.
    pub async fn start_scanning(&mut self) -> Option<ButtplugClientError> {
        self.send_message_expect_ok(&ButtplugMessageUnion::StartScanning(StartScanning::new()))
            .await
    }

    // Send message to the internal event loop. Mostly for handling boilerplate
    // around possible send errors.
    async fn send_internal_message(
        &mut self,
        msg: ButtplugInternalClientMessage
    ) -> Option<ButtplugClientError> {
        // If we're running the event loop, we should have a message_sender.
        // Being connected to the server doesn't matter here yet because we use
        // this function in order to connect also.
        if let Some(sender) = &self.message_sender {
            sender.send(msg).await;
            None
        } else {
            Some(ButtplugClientError::ButtplugClientConnectorError(
                ButtplugClientConnectorError {
                    message: "Client event loop not running.".to_string(),
                },
            ))
        }
    }

    // Sends a ButtplugMessage from client to server. Expects to receive a
    // ButtplugMessage back from the server.
    async fn send_message(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        // If we're not connected to a server, there's nowhere to send a
        // ButtplugMessage to, so error out early.
        if !self.connected {
            return Err(ButtplugClientError::ButtplugClientConnectorError(
                ButtplugClientConnectorError {
                    message: "Client not Connected.".to_string(),
                },
            ));
        }
        // Create a future to pair with the message being resolved.
        let fut = ButtplugClientMessageFuture::default();
        let internal_msg = ButtplugInternalClientMessage::Message((
            msg.clone(),
            fut.get_state_clone(),
        ));
        // Make sure we can send the message. If we send without a problem, then
        // wait on the future we paired with the message to return.
        //
        // TODO How we'd get here without the internal loop running is a good
        // question, so we may be able to simplify this and assume we can unwrap.
        match self.send_internal_message(internal_msg).await {
            Some(err) => Err(err),
            None => Ok(fut.await)
        }
    }

    // Sends a ButtplugMessage from client to server. Expects to receive an [Ok]
    // type ButtplugMessage back from the server.
    async fn send_message_expect_ok(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Option<ButtplugClientError> {
        let msg = self.send_message(msg).await;
        match msg.unwrap() {
            ButtplugMessageUnion::Ok(_) => None,
            _ => Some(ButtplugClientError::ButtplugError(
                ButtplugError::ButtplugMessageError(ButtplugMessageError {
                    message: "Got non-Ok message back".to_string(),
                }),
            )),
        }
    }

    /// Produces a future that will wait for a set of events from the
    /// internal loop. Returns once any number of events is received.
    ///
    /// This should be called whenever the client isn't doing anything
    /// otherwise, so we can respond to unexpected updates from the server, such
    /// as devices connections/disconnections, log messages, etc... This is
    /// basically what event handlers in C# and JS would deal with, but we're in
    /// Rust so this requires us to be slightly more explicit.
    pub async fn wait_for_event(&mut self) -> Vec<ButtplugClientEvent> {
        if self.message_sender.is_none() {
            panic!("Cannot wait for events before internal loop is running!");
        }
        debug!("Client waiting for event.");
        let mut events = vec!();
        if let Some(ref mut receiver) = self.event_receiver {
            match receiver.next().await.unwrap() {
                ButtplugMessageUnion::ScanningFinished(_) => {}
                ButtplugMessageUnion::DeviceList(_msg) => {
                    for info in _msg.devices.iter() {
                        // Calling unwrap here is fine, because we can't even get
                        // events if the internal loop isn't already running.
                        let device =
                            ButtplugClientDevice::from((&info.clone(),
                                                        self.message_sender.as_ref().unwrap().clone()));
                        self.devices.push(device.clone());
                        events.push(ButtplugClientEvent::DeviceAdded(device));
                    }
                }
                ButtplugMessageUnion::DeviceAdded(_msg) => {
                    info!("Got a device added message!");
                    // Calling unwrap here is fine, because we can't even get
                    // events if the internal loop isn't already running.
                    let device = ButtplugClientDevice::from((&_msg,
                                                             self.message_sender.as_ref().unwrap().clone()));
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
        } else {
            vec!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::client::connector::ButtplugEmbeddedClientConnector;
    use async_std::task;
    use env_logger;

    async fn connect_test_client() -> ButtplugClient {
        let mut client = ButtplugClient::new("Test Client");
        let fut_loop = client.get_loop();
        task::spawn(async move {
            fut_loop.await;
        });
        assert!(client
                .connect(ButtplugEmbeddedClientConnector::new("Test Server", 0))
                .await
                .is_none());
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
            assert!(client.disconnect().is_none());
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
            assert!(client.start_scanning().await.is_none());
        });
    }

    #[test]
    fn test_scanning_finished() {
        task::block_on(async {
            let mut client = connect_test_client().await;
            assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
            assert!(client.start_scanning().await.is_none());
        });
    }

    // Failure on server version error is unit tested in server.
}
