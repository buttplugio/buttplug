// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Communications API for accessing Buttplug Servers

pub mod connectors;
pub mod device;
pub mod internal;

use connectors::{
    ButtplugClientConnectionFuture, ButtplugClientConnector, ButtplugClientConnectorError,
};
use device::ButtplugClientDevice;
use internal::{client_event_loop, ButtplugClientMessage};

use crate::{
    core::{
        errors::{
            ButtplugDeviceError, ButtplugError, ButtplugHandshakeError, ButtplugMessageError,
        },
        messages::{
            ButtplugMessage, ButtplugMessageUnion, DeviceMessageInfo, LogLevel, RequestDeviceList,
            RequestServerInfo, StartScanning,
        },
    },
    util::future::{ButtplugFuture, ButtplugMessageFuture},
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
/// Clients are created by the [ButtplugClient::run()] method, which also
/// handles spinning up the event loop and connecting the client to the server.
/// Closures passed to the run() method can access and use the Client object.
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
    // Storage for events received when checking for events during
    // non-wait_for_event calls.
    events: Vec<ButtplugClientEvent>,
}

unsafe impl Sync for ButtplugClient {}
unsafe impl Send for ButtplugClient {}

impl ButtplugClient {
    /// Runs the client event loop.
    ///
    /// Given a client name, a connector, and a function that takes the client
    /// and returns an future (since we can't have async closures yet), this
    /// function
    ///
    /// - creates a ButtplugClient instance, and connects it to the server via the
    /// connector instance that was passed in.
    /// - passes it to the `func` argument to create the application [Future]
    /// - returns a [Future] that joins the client event loop future and
    /// the client application future.
    ///
    /// # Parameters
    ///
    /// - `name`: Name of the client, see [ButtplugClient::client_name]
    /// - `connector`: Connector instance for handling connection and communication
    /// with the Buttplug Server
    /// - `func`: Function that takes the client instance, and returns a future
    /// for what the application will be doing with the client instance.
    ///
    /// # Returns
    ///
    /// Ok(()) if connection is successful and closure executes correctly,
    /// Err(ButtplugClientError) if connection with the server fails.
    ///
    /// # Examples
    ///
    /// ```
    /// #[cfg(feature = "server")]
    /// use buttplug::client::{ButtplugClient, connectors::ButtplugEmbeddedClientConnector};
    ///
    /// #[cfg(feature = "server")]
    /// futures::executor::block_on(async {
    ///     ButtplugClient::run("Test Client", ButtplugEmbeddedClientConnector::new("Test Server", 0), |mut client| {
    ///         async move {
    ///             println!("Are we connected? {}", client.connected());
    ///         }
    ///     }).await;
    /// });
    /// ```
    pub fn run<F, T>(
        name: &str,
        connector: impl ButtplugClientConnector + 'static,
        func: F,
    ) -> impl Future<Output = ButtplugClientResult>
    where
        F: FnOnce(ButtplugClient) -> T,
        T: Future<Output = ()>,
    {
        debug!("Run called!");
        let (event_sender, event_receiver) = channel(256);
        let (message_sender, message_receiver) = channel(256);
        let mut client = ButtplugClient {
            client_name: name.to_string(),
            server_name: None,
            event_receiver,
            message_sender,
            connected: true,
            events: vec![],
        };
        let app_future = async move {
            client.connect(connector).await?;
            func(client).await;
            Ok(())
        };
        async move {
            let internal_loop_future = client_event_loop(event_sender, message_receiver);
            app_future.race(internal_loop_future).await
        }
    }

    /// Connects and runs handshake flow with
    /// [crate::server::server::ButtplugServer], either local or remote.
    ///
    /// Called by run() while spinning up the event loop. Tries to connect to a
    /// server via the given [ButtplugClientConnector] struct. If connection is
    /// successful, also runs the handshake flow and retrieves a list of
    /// currently connected devices. These devices will be emitted as
    /// [ButtplugClientEvent::DeviceAdded] events next time
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
    pub(self) async fn connect(
        &mut self,
        connector: impl ButtplugClientConnector + 'static,
    ) -> ButtplugClientResult {
        debug!("Running client connection.");

        // Send the connector to the internal loop for management. Once we throw
        // the connector over, the internal loop will handle connecting and any
        // further communications with the server, if connection is successful.
        let fut = ButtplugClientConnectionFuture::default();
        let msg = ButtplugClientMessage::Connect(Box::new(connector), fut.get_state_clone());
        self.send_internal_message(msg).await?;

        debug!("Waiting on internal loop to connect");
        if let Err(e) = fut.await {
            return Err(ButtplugClientError::from(e));
        }

        info!("Client connected to server, running handshake.");
        self.handshake().await
    }

    // Runs the handshake flow with the server.
    //
    // Sends over RequestServerInfo, gets back ServerInfo, sets up ping timer if
    // needed.
    async fn handshake(&mut self) -> ButtplugClientResult {
        info!("Running handshake with server.");
        match self
            .send_message(&RequestServerInfo::new(&self.client_name, 1).into())
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
                            .await?;
                    }
                    Ok(())
                } else {
                    self.disconnect().await?;
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

    /// Status of the client connection.
    ///
    /// # Returns
    /// Returns true if client is currently connected to server, false otherwise.
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// Disconnects from server, if connected.
    ///
    /// # Returns
    ///
    /// Ok(()) if disconnection is successful, Err(ButtplugClientError) if
    /// disconnection fails. It can be assumed that even on failure, the client
    /// will be disconnected.
    pub async fn disconnect(&mut self) -> ButtplugClientResult {
        // Send the connector to the internal loop for management. Once we throw
        // the connector over, the internal loop will handle connecting and any
        // further communications with the server, if connection is successful.
        let fut = ButtplugClientConnectionFuture::default();
        let msg = ButtplugClientMessage::Disconnect(fut.get_state_clone());
        self.send_internal_message(msg).await?;
        self.connected = false;
        Ok(())
    }

    /// Tells server to start scanning for devices.
    ///
    /// # Returns
    ///
    /// Ok(()) if request is successful, Err([ButtplugClientError]) if request
    /// fails due to issues with DeviceManagers on the server, disconnection,
    /// etc.
    pub async fn start_scanning(&mut self) -> ButtplugClientResult {
        self.send_message_expect_ok(&ButtplugMessageUnion::StartScanning(
            StartScanning::default(),
        ))
        .await
    }

    // Send message to the internal event loop. Mostly for handling boilerplate
    // around possible send errors.
    async fn send_internal_message(
        &mut self,
        msg: ButtplugClientMessage,
    ) -> Result<(), ButtplugClientConnectorError> {
        // Since we're using async_std channels, if we send a message and the
        // event loop has shut down, we may never know (and therefore possibly
        // block infinitely) if we don't check the status of an event loop
        // receiver to see if it's returned None. Always run connection/event
        // checks before sending messages to the event loop.
        self.check_for_events().await?;

        // If we're running the event loop, we should have a message_sender.
        // Being connected to the server doesn't matter here yet because we use
        // this function in order to connect also.
        self.message_sender.send(msg).await;
        Ok(())
    }

    // Sends a ButtplugMessage from client to server. Expects to receive a
    // ButtplugMessage back from the server.
    async fn send_message(
        &mut self,
        msg: &ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        // Create a future to pair with the message being resolved.
        let fut = ButtplugMessageFuture::default();
        let internal_msg = ButtplugClientMessage::Message((msg.clone(), fut.get_state_clone()));

        // Send message to internal loop and wait for return.
        self.send_internal_message(internal_msg).await?;
        Ok(fut.await)
    }

    // Sends a ButtplugMessage from client to server. Expects to receive an [Ok]
    // type ButtplugMessage back from the server.
    async fn send_message_expect_ok(&mut self, msg: &ButtplugMessageUnion) -> ButtplugClientResult {
        match self.send_message(msg).await? {
            ButtplugMessageUnion::Ok(_) => Ok(()),
            _ => Err(ButtplugClientError::from(ButtplugMessageError::new(
                "Got non-Ok message back",
            ))),
        }
    }

    async fn check_for_events(&mut self) -> Result<(), ButtplugClientConnectorError> {
        if !self.connected {
            return Err(ButtplugClientConnectorError::new("Client not connected."));
        }
        while !self.event_receiver.is_empty() {
            match self.event_receiver.next().await {
                Some(msg) => self.events.push(msg),
                None => {
                    self.connected = false;
                    // If we got None, this means the internal loop stopped and our
                    // sender was dropped. We should consider this a disconnect.
                    self.events.push(ButtplugClientEvent::ServerDisconnect);
                    return Err(ButtplugClientConnectorError::new("Client not connected."));
                }
            }
        }
        Ok(())
    }

    /// Produces a future that will wait for a set of events from the
    /// internal loop. Returns every time an event is received.
    ///
    /// This should be called whenever the client isn't doing anything
    /// otherwise, so we can respond to unexpected updates from the server, such
    /// as devices connections/disconnections, log messages, etc... This is
    /// basically what event handlers in C# and JS would deal with, but we're in
    /// Rust so this requires us to be slightly more explicit.
    ///
    /// # Returns
    ///
    /// Ok([ButtplugClientEvent]) if event is received successfully,
    /// Err([ButtplugClientConnectorError]) if waiting fails due to server/client
    /// disconnection.

    pub async fn wait_for_event(
        &mut self,
    ) -> Result<ButtplugClientEvent, ButtplugClientConnectorError> {
        debug!("Client waiting for event.");
        if !self.connected {
            return Err(ButtplugClientConnectorError::new("Client not connected."));
        }
        Ok({
            if !self.events.is_empty() {
                self.events.pop().unwrap()
            } else {
                match self.event_receiver.next().await {
                    Some(msg) => msg,
                    None => {
                        // If we got None, this means the internal loop stopped and our
                        // sender was dropped. We should consider this a disconnect.
                        self.connected = false;
                        ButtplugClientEvent::ServerDisconnect
                    }
                }
            }
        })
    }

    /// Retreives a list of devices. This requires communication with the Event
    /// Loop, which is why it is an asynchronous function.
    ///
    /// # Returns
    ///
    /// Ok(Vec<[ButtplugClientDevice]>) if successful,
    /// Err([ButtplugClientConnectorError]) if the server has disconnected.
    pub async fn devices(
        &mut self,
    ) -> Result<Vec<ButtplugClientDevice>, ButtplugClientConnectorError> {
        info!("Request devices from inner loop!");
        let fut = ButtplugFuture::<Vec<ButtplugClientDevice>>::default();
        let msg = ButtplugClientMessage::RequestDeviceList(fut.get_state_clone());
        info!("Sending device request to inner loop!");
        self.send_internal_message(msg).await?;
        info!("Waiting for device list return!");
        Ok(fut.await)
    }
}

#[cfg(all(test, feature = "server"))]
mod test {
    use super::ButtplugClient;
    use crate::{
        client::connectors::{
            ButtplugClientConnector, ButtplugClientConnectorError, ButtplugEmbeddedClientConnector,
        },
        core::messages::ButtplugMessageUnion,
        util::future::ButtplugMessageStateShared,
    };
    use async_std::{
        future::Future,
        sync::{channel, Receiver},
        task,
    };
    use async_trait::async_trait;
    use env_logger;

    async fn connect_test_client<F, T>(func: F)
    where
        F: FnOnce(ButtplugClient) -> T,
        T: Future<Output = ()>,
    {
        let _ = env_logger::builder().is_test(true).try_init();
        assert!(ButtplugClient::run(
            "Test Client",
            ButtplugEmbeddedClientConnector::new("Test Server", 0),
            func
        )
        .await
        .is_ok());
    }

    #[derive(Default)]
    struct ButtplugFailingConnector {}

    #[async_trait]
    impl ButtplugClientConnector for ButtplugFailingConnector {
        async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
            Err(ButtplugClientConnectorError::new("Always fails"))
        }

        async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError> {
            Err(ButtplugClientConnectorError::new("Always fails"))
        }

        async fn send(&mut self, _msg: &ButtplugMessageUnion, _state: &ButtplugMessageStateShared) {
        }

        fn get_event_receiver(&mut self) -> Receiver<ButtplugMessageUnion> {
            // This will panic if we've already taken the receiver.
            let (_send, recv) = channel(256);
            recv
        }
    }

    #[test]
    fn test_failing_connection() {
        let _ = env_logger::builder().is_test(true).try_init();
        task::block_on(async {
            assert!(ButtplugClient::run(
                "Test Client",
                ButtplugFailingConnector::default(),
                |_| { async {} }
            )
            .await
            .is_err());
        });
    }

    #[test]
    fn test_disconnect_status() {
        task::block_on(async {
            connect_test_client(|mut client| {
                async move {
                    assert!(client.disconnect().await.is_ok());
                    assert!(!client.connected());
                }
            })
            .await;
        });
    }

    #[test]
    fn test_double_disconnect() {
        task::block_on(async {
            connect_test_client(|mut client| {
                async move {
                    assert!(client.disconnect().await.is_ok());
                    assert!(client.disconnect().await.is_err());
                }
            })
            .await;
        });
    }

    #[test]
    fn test_connect_init() {
        task::block_on(async {
            connect_test_client(|client| {
                async move {
                    assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
                }
            })
            .await;
        });
    }

    #[test]
    fn test_start_scanning() {
        task::block_on(async {
            connect_test_client(|mut client| {
                async move {
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
