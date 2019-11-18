// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Communications API for accessing Buttplug Servers

pub mod connector;
pub mod device;
pub mod internal;
mod messagesorter;

use connector::{
    ButtplugClientConnectionFuture, ButtplugClientConnector, ButtplugClientConnectorError,
};
use device::ButtplugClientDevice;
use internal::{
    client_event_loop, ButtplugClientFuture, ButtplugClientMessage, ButtplugClientMessageFuture,
};

use crate::core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
    messages::{
        ButtplugMessage, ButtplugMessageUnion, DeviceMessageInfo, LogLevel, RequestDeviceList,
        RequestServerInfo, StartScanning,
    },
};

use async_std::{
    prelude::FutureExt,
    sync::{channel, Receiver, Sender},
};
use futures::{Future, StreamExt};
use std::{error::Error, fmt};

type ButtplugClientResult<T = ()> = Result<T, ButtplugClientError>;

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

impl From<ButtplugClientConnectorError> for ButtplugClientError {
    fn from(error: ButtplugClientConnectorError) -> Self {
        ButtplugClientError::ButtplugClientConnectorError(error)
    }
}

impl From<ButtplugMessageError> for ButtplugClientError {
    fn from(error: ButtplugMessageError) -> Self {
        ButtplugClientError::ButtplugError(ButtplugError::ButtplugMessageError(error))
    }
}

impl From<ButtplugDeviceError> for ButtplugClientError {
    fn from(error: ButtplugDeviceError) -> Self {
        ButtplugClientError::ButtplugError(ButtplugError::ButtplugDeviceError(error))
    }
}

/// Enum representing different events that can be emitted by a client.
///
/// These events are created by the server and sent to the client, and represent
/// unrequested actions that the client will need to respond to, or that
/// applications using the client may be interested in.
pub enum ButtplugClientEvent {
    /// Emitted when a scanning session (started via a StartScanning call on
    /// [ButtplugClient]) has finished.
    ScanningFinished,
    /// Emitted when a device has been added to the server. Includes a
    /// [ButtplugClientDevice] object representing the device.
    DeviceAdded(ButtplugClientDevice),
    /// Emitted when a device has been removed from the server. Includes a
    /// [ButtplugClientDevice] object representing the device.
    DeviceRemoved(DeviceMessageInfo),
    /// Emitted when log messages are sent from the server.
    Log(LogLevel, String),
    /// Emitted when a client has not pinged the server in a sufficient amount
    /// of time.
    PingTimeout,
    /// Emitted when a client connector detects that the server has
    /// disconnected.
    ServerDisconnect,
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
pub struct ButtplugClient {
    /// The client name. Depending on the connection type and server being used,
    /// this name is sometimes shown on the server logs or GUI.
    pub client_name: String,
    /// The server name. Once connected, this contains the name of the server,
    /// so we can know what we're connected to.
    pub server_name: Option<String>,
    // Sender to relay messages to the internal client loop
    message_sender: Sender<ButtplugClientMessage>,
    // Receives event notifications from the ButtplugClientLoop
    event_receiver: Receiver<ButtplugClientEvent>,
    // True if the connector is currently connected, and handshake was
    // successful.
    connected: bool,
}

unsafe impl Sync for ButtplugClient {}
unsafe impl Send for ButtplugClient {}

impl ButtplugClient {
    /// Runs the client event loop.
    ///
    /// Given a client name and a function that takes the client and returns an
    /// future (since we can't have async closures yet), this function
    ///
    /// - creates a ButtplugClient instance
    /// - passes it to the `func` argument to create the application [Future]
    /// - returns a [Future] with a [join] for the client event loop future and
    /// the client application future.
    ///
    /// # Parameters
    ///
    /// - `name`: Name of the client, see [ButtplugClient::client_name]
    /// - `func`: Function that takes the client instance, and returns a future
    /// for what the application will be doing with the client instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use buttplug::client::{ButtplugClient, connector::ButtplugEmbeddedClientConnector};
    ///
    /// futures::executor::block_on(async {
    ///     ButtplugClient::run("Test Client", |mut client| {
    ///         async move {
    ///             client
    ///                 .connect(ButtplugEmbeddedClientConnector::new("Test Server", 0))
    ///                 .await;
    ///             println!("Are we connected? {}", client.connected());
    ///         }
    ///     }).await;
    /// });
    /// ```
    pub fn run<F, T>(name: &str, func: F) -> impl Future
    where
        F: FnOnce(ButtplugClient) -> T,
        T: Future<Output = ()>,
    {
        debug!("Run called!");
        let (event_sender, event_receiver) = channel(256);
        let (message_sender, message_receiver) = channel(256);
        let client = ButtplugClient {
            client_name: name.to_string(),
            server_name: None,
            event_receiver,
            message_sender,
            connected: false,
        };
        let app_future = func(client);
        async move {
            let internal_loop_future = client_event_loop(event_sender, message_receiver);
            app_future.join(internal_loop_future).await;
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
    ) -> ButtplugClientResult {
        debug!("Running client connection.");

        // Send the connector to the internal loop for management. Once we throw
        // the connector over, the internal loop will handle connecting and any
        // further communications with the server, if connection is successful.
        let fut = ButtplugClientConnectionFuture::default();
        let msg = ButtplugClientMessage::Connect(Box::new(connector), fut.get_state_clone());
        self.send_internal_message(msg).await;

        debug!("Waiting on internal loop to connect");
        fut.await?;

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
    async fn handshake(&mut self) -> ButtplugClientResult {
        info!("Running handshake with server.");
        match self
            .send_message(&RequestServerInfo::new(&self.client_name, 1).as_union())
            .await
        {
            Ok(msg) => {
                debug!("Got ServerInfo return.");
                if let ButtplugMessageUnion::ServerInfo(server_info) = msg {
                    info!("Connected to {}", server_info.server_name);
                    self.server_name = Option::Some(server_info.server_name);
                    // TODO Handle ping time in the internal event loop

                    // Get currently connected devices. The event loop will
                    // handle sending the message and getting the return, and
                    // will send the client updates as events.
                    let msg = self
                        .send_message(&ButtplugMessageUnion::RequestDeviceList(
                            RequestDeviceList::default(),
                        ))
                        .await?;
                    if let ButtplugMessageUnion::DeviceList(m) = msg {
                        self.send_internal_message(ButtplugClientMessage::HandleDeviceList(m))
                            .await;
                    }
                    Ok(())
                } else {
                    // TODO Should disconnect here.
                    Err(ButtplugClientError::ButtplugError(
                        ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError {
                            message: "Did not receive expected ServerInfo or Error messages."
                                .to_string(),
                        }),
                    ))
                }
            }
            // TODO Error message case may need to be implemented here when
            // we aren't only using embedded connectors.
            Err(e) => Err(e),
        }
    }

    /// Returns true if client is currently connected to server.
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// Disconnects from server, if connected.
    pub async fn disconnect(&mut self) -> ButtplugClientResult {
        // Send the connector to the internal loop for management. Once we throw
        // the connector over, the internal loop will handle connecting and any
        // further communications with the server, if connection is successful.
        let fut = ButtplugClientConnectionFuture::default();
        let msg = ButtplugClientMessage::Disconnect(fut.get_state_clone());
        self.send_internal_message(msg).await;
        self.connected = false;
        Ok(())
    }

    /// Tells server to start scanning for devices.
    pub async fn start_scanning(&mut self) -> ButtplugClientResult {
        self.send_message_expect_ok(&ButtplugMessageUnion::StartScanning(StartScanning::default()))
            .await
    }

    // Send message to the internal event loop. Mostly for handling boilerplate
    // around possible send errors.
    async fn send_internal_message(&mut self, msg: ButtplugClientMessage) {
        // If we're running the event loop, we should have a message_sender.
        // Being connected to the server doesn't matter here yet because we use
        // this function in order to connect also.
        self.message_sender.send(msg).await;
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
        let internal_msg = ButtplugClientMessage::Message((msg.clone(), fut.get_state_clone()));

        // Send message to internal loop and wait for return.
        self.send_internal_message(internal_msg).await;
        Ok(fut.await)
    }

    // Sends a ButtplugMessage from client to server. Expects to receive an [Ok]
    // type ButtplugMessage back from the server.
    async fn send_message_expect_ok(&mut self, msg: &ButtplugMessageUnion) -> ButtplugClientResult {
        let msg = self.send_message(msg).await;
        match msg.unwrap() {
            ButtplugMessageUnion::Ok(_) => Ok(()),
            _ => Err(ButtplugClientError::from(ButtplugMessageError::new(
                "Got non-Ok message back",
            ))),
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
        debug!("Client waiting for event.");
        let mut events = vec![];
        match self.event_receiver.next().await {
            Some(msg) => events.push(msg),
            None => {
                // If we got None, this means the internal loop stopped and our
                // sender was dropped. We should consider this a disconnect.
                events.push(ButtplugClientEvent::ServerDisconnect)
            }
        };
        events
    }

    pub async fn devices(&mut self) -> Vec<ButtplugClientDevice> {
        info!("Request devices from inner loop!");
        let fut = ButtplugClientFuture::<Vec<ButtplugClientDevice>>::default();
        let msg = ButtplugClientMessage::RequestDeviceList(fut.get_state_clone());
        info!("Sending device request to inner loop!");
        self.send_internal_message(msg).await;
        info!("Waiting for device list return!");
        fut.await
    }
}

#[cfg(test)]
mod test {
    use super::ButtplugClient;
    use crate::{
        client::{
            connector::{
                ButtplugClientConnector, ButtplugClientConnectorError,
                ButtplugEmbeddedClientConnector,
            },
            internal::ButtplugClientMessageStateShared,
        },
        core::messages::ButtplugMessageUnion,
    };
    use async_std::{
        sync::{channel, Receiver},
        task,
    };
    use async_trait::async_trait;
    use env_logger;

    async fn connect_test_client(client: &mut ButtplugClient) {
        let _ = env_logger::builder().is_test(true).try_init();
        assert!(client
            .connect(ButtplugEmbeddedClientConnector::new("Test Server", 0))
            .await
            .is_ok());
        assert!(client.connected());
    }

    #[derive(Default)]
    struct ButtplugFailingClientConnector {}

    #[async_trait]
    impl ButtplugClientConnector for ButtplugFailingClientConnector {
        async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
            Err(ButtplugClientConnectorError::new("Always fails"))
        }

        async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError> {
            Err(ButtplugClientConnectorError::new("Always fails"))
        }

        async fn send(
            &mut self,
            _msg: &ButtplugMessageUnion,
            _state: &ButtplugClientMessageStateShared,
        ) {
        }

        fn get_event_receiver(&mut self) -> Receiver<ButtplugMessageUnion> {
            // This will panic if we've already taken the receiver.
            let (_send, recv) = channel(256);
            recv
        }
    }

    #[test]
    fn test_failing_connection() {
        task::block_on(async {
            ButtplugClient::run("Test Client", |mut client| {
                async move {
                    assert!(client
                        .connect(ButtplugFailingClientConnector::default())
                        .await
                        .is_err());
                    assert!(!client.connected());
                }
            })
            .await;
        });
    }

    #[test]
    fn test_connect_status() {
        task::block_on(async {
            ButtplugClient::run("Test Client", |mut client| {
                async move {
                    connect_test_client(&mut client).await;
                }
            })
            .await;
        });
    }

    #[test]
    fn test_disconnect_status() {
        task::block_on(async {
            ButtplugClient::run("Test Client", |mut client| {
                async move {
                    connect_test_client(&mut client).await;
                    assert!(client.disconnect().await.is_ok());
                    assert!(!client.connected());
                }
            })
            .await;
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
            ButtplugClient::run("Test Client", |mut client| {
                async move {
                    connect_test_client(&mut client).await;
                    assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
                }
            })
            .await;
        });
    }

    #[test]
    fn test_start_scanning() {
        task::block_on(async {
            ButtplugClient::run("Test Client", |mut client| {
                async move {
                    connect_test_client(&mut client).await;
                    assert!(client.start_scanning().await.is_ok());
                }
            })
            .await;
        });
    }

    // #[test]
    // fn test_scanning_finished() {
    //     task::block_on(async {
    //         let mut client = connect_test_client().await;
    //         assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
    //         assert!(client.start_scanning().await.is_none());
    //     });
    // }

    // Failure on server version error is unit tested in server.
}
