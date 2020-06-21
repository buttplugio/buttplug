// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Communications API for accessing Buttplug Servers
mod client_message_sorter;
pub mod device;
pub mod internal;

use device::ButtplugClientDevice;
use internal::{client_event_loop, ButtplugClientRequest, ButtplugClientDeviceInternal};

use crate::{
  connector::{
    ButtplugConnector, ButtplugConnectorError, ButtplugConnectorFuture,
  },
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
    messages::{
      ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage,
      ButtplugMessageSpecVersion, DeviceMessageInfo, LogLevel, RequestDeviceList,
      RequestServerInfo, StartScanning,
    },
  },
  util::{
    future::{ButtplugFuture, ButtplugFutureStateShared},
    async_manager,
  }
};
use async_channel::Sender;
use futures::{
  future::{self, BoxFuture, Future},
  StreamExt,
  FutureExt,
};
use std::{
  error::Error,
  fmt,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use dashmap::DashMap;
use tracing::{span::Span, Level};
use tracing_futures::Instrument;

/// Result type used inside the client module.
///
/// When communicating inside the client module, we'll usually only receive
/// errors related to the connector. Buttplug
/// [Error][crate::core::messages::Error] messages will still be valid, because
/// they're coming from the server.
type ButtplugInternalClientResult<T = ()> = Result<T, ButtplugConnectorError>;
/// Result type used for public APIs.
///
/// Allows us to differentiate between an issue with the connector (as a
/// [ButtplugConnectorError]) and an issue within Buttplug (as a
/// [ButtplugError]).
type ButtplugClientResult<T = ()> = Result<T, ButtplugClientError>;
type ButtplugClientResultFuture<T = ()> = BoxFuture<'static, ButtplugClientResult<T>>;

/// Result type used for passing server responses.
pub type ButtplugInternalClientMessageResult =
  ButtplugInternalClientResult<ButtplugCurrentSpecServerMessage>;
pub type ButtplugInternalClientMessageResultFuture =
  BoxFuture<'static, ButtplugInternalClientMessageResult>;
/// Future state type for returning server responses across futures.
pub(crate) type ButtplugClientMessageStateShared =
  ButtplugFutureStateShared<ButtplugInternalClientMessageResult>;
/// Future type that expects server responses.
pub(crate) type ButtplugClientMessageFuture = ButtplugFuture<ButtplugInternalClientMessageResult>;

/// Future state for messages sent from the client that expect a server
/// response.
///
/// When a message is sent from the client and expects a response from the
/// server, we'd like to know when that response arrives, and usually we'll want
/// to wait for it. We can do so by creating a future that will be resolved when
/// a response is received from the server.
///
/// To do this, we build a [ButtplugFuture], then take its waker and pass it
/// along with the message we send to the connector, using the
/// [ButtplugClientMessageFuturePair] type. We can then expect the connector to
/// get the response from the server, match it with our message (using something
/// like the
/// [ClientConnectorMessageSorter][crate::connector::ClientConnectorMessageSorter]),
/// and set the reply in the waker we've sent along. This will resolve the
/// future we're waiting on and allow us to continue execution.
pub struct ButtplugClientMessageFuturePair {
  pub msg: ButtplugCurrentSpecClientMessage,
  pub waker: ButtplugClientMessageStateShared,
}

impl ButtplugClientMessageFuturePair {
  pub fn new(
    msg: ButtplugCurrentSpecClientMessage,
    waker: ButtplugClientMessageStateShared,
  ) -> Self {
    Self { msg, waker }
  }
}

/// Represents all of the different types of errors a ButtplugClient can return.
///
/// Clients can return two types of errors:
///
/// - [ButtplugConnectorError], which means there was a problem with the
/// connection between the client and the server, like a network connection
/// issue.
/// - [ButtplugError], which is an error specific to the Buttplug Protocol.
#[derive(Debug, Clone)]
pub enum ButtplugClientError {
  /// Connector error
  ButtplugConnectorError(ButtplugConnectorError),
  /// Protocol error
  ButtplugError(ButtplugError),
}

impl fmt::Display for ButtplugClientError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      ButtplugClientError::ButtplugError(ref e) => e.fmt(f),
      ButtplugClientError::ButtplugConnectorError(ref e) => e.fmt(f),
    }
  }
}

impl Error for ButtplugClientError {
  fn source(&self) -> Option<&(dyn Error + 'static)> {
    None
  }
}

impl From<ButtplugConnectorError> for ButtplugClientError {
  fn from(error: ButtplugConnectorError) -> Self {
    ButtplugClientError::ButtplugConnectorError(error)
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
  /// The server name that we're current connected to.
  pub server_name: String,
  // Sender to relay messages to the internal client loop
  message_sender: Sender<ButtplugClientRequest>,
  // True if the connector is currently connected, and handshake was
  // successful.
  connected: Arc<AtomicBool>,
  _client_span: Span,
  device_map: Arc<DashMap<u32, ButtplugClientDeviceInternal>>,
}

unsafe impl Send for ButtplugClient {}
// Not actually sure this should be sync, but trying to call handshake breaks
// without it.
unsafe impl Sync for ButtplugClient {}

impl ButtplugClient {
  pub fn connect<ConnectorType>(
    name: &str,
    mut connector: ConnectorType,
  ) -> BoxFuture<'static, Result<(Self, impl StreamExt<Item=ButtplugClientEvent>), ButtplugClientError>>
  where
  ConnectorType: ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage> + 'static {
    trace!("run() called, creating client future.");
    let client_name = name.to_string();
    Box::pin(async move {
      let span = span!(Level::INFO, "Client");
      let _client_span = span.enter();
      info!("Connecting to server.");
      let connector_receiver = connector.connect().await.map_err(|e| {
        error!("Connection to server failed: {:?}", e);
        let err: ButtplugClientError = e.into();
        err
      })?;
      info!("Connection to server succeeded.");
      let (client_event_loop_fut, 
        device_map_reader, 
        message_sender,
        event_channel) = client_event_loop(
        connector,
        connector_receiver,
      );

      let client_event_receiver = event_channel.clone();
      let mut disconnect_event_receiver = event_channel.clone();
      let connected_status = Arc::new(AtomicBool::new(true));
      let connected_status_clone = connected_status.clone();

      // Start the event loop before we run the handshake.
      async_manager::spawn(async move {
        let disconnect_fut = async move {
          loop {
            if let Some(ButtplugClientEvent::ServerDisconnect) = disconnect_event_receiver.next().await {
              connected_status.store(false, Ordering::SeqCst);
              break;
            }
          }
          Result::<(), ButtplugClientError>::Ok(())
        }.instrument(tracing::info_span!("Client Disconnect Loop"));
        // If we disconnect, we'll also stop the client event loop. If the
        // client event loop stops, we don't care about listening for disconnect
        // anymore.
        select! {
          _ = client_event_loop_fut.fuse() => (),
          _ = disconnect_fut.fuse() => (),
        };
      }.instrument(tracing::info_span!("Client Loop Span"))).unwrap();
      let client = ButtplugClient::create_client(
        &client_name,
        connected_status_clone,
        message_sender,
        device_map_reader,
        span.clone(),
      ).await?;
      Ok((client, client_event_receiver))
    })
  }

  /// Convenience function for creating in-process connectors.
  ///
  /// Creates a [ButtplugClient] event loop, with an in-process connector with
  /// all device managers that ship with the library and work on the current
  /// platform added to it already. Takes a maximum ping time to build the
  /// server with, other parameters match `run()`.
  ///
  /// # When To Use This Instead of `run()`
  ///
  /// If you just want to build a quick example and save yourself a few use
  /// statements and setup, this will get you going. For anything *production*,
  /// we recommend using `run()` as you will have more control over what
  /// happens. This method may gain/lose device comm managers at any time.
  ///
  /// # The Device I Want To Use Doesn't Show Up
  ///
  /// If you are trying to use this method to create your client, and do not see
  /// the devices you want, there are a couple of things to check:
  ///
  /// - Are you on a platform that the device communication manager supports?
  ///   For instance, we only support XInput on windows.
  /// - Did the developers add a new Device CommunicationManager type and forget
  ///   to add it to this method? _It's more likely than you think!_ [File a
  ///   bug](https://github.com/buttplugio/buttplug-rs/issues).
  ///
  /// # Errors
  ///
  /// If the library was compiled without any device managers, the
  /// [ButtplugClient] will have nothing to do. This is considered a
  /// catastrophic failure and the library will return an error.
  ///
  /// If the library is using outside device managers, it is recommended to
  /// build your own connector, add your device manager to those, and use the
  /// `run()` method to pass it in.
  pub fn connect_in_process(
    name: &str,
    max_ping_time: u64
  ) -> impl Future<Output = Result<(Self, impl StreamExt<Item = ButtplugClientEvent>), ButtplugClientError>> {
    use crate::connector::ButtplugInProcessClientConnector;

    let mut connector =
      ButtplugInProcessClientConnector::new("Default In Process Server", max_ping_time);
    #[cfg(feature = "btleplug-manager")]
    {
      use crate::server::comm_managers::btleplug::BtlePlugCommunicationManager;
      connector
        .server_ref()
        .add_comm_manager::<BtlePlugCommunicationManager>();
    }
    #[cfg(all(feature = "xinput", target_os = "windows"))]
    {
      use crate::server::comm_managers::xinput::XInputDeviceCommunicationManager;
      connector
        .server_ref()
        .add_comm_manager::<XInputDeviceCommunicationManager>();
    }
    ButtplugClient::connect(name, connector)
  }

  /// Creates the ButtplugClient instance and tries to establish a connection.
  ///
  /// Takes all of the components needed to build a [ButtplugClient], creates
  /// the struct, then tries to run connect and execute the Buttplug protocol
  /// handshake. Will return a connected and ready to use ButtplugClient is all
  /// goes well.
  async fn create_client(client_name: &str,
    connected_status: Arc<AtomicBool>, 
    message_sender: Sender<ButtplugClientRequest>, 
    device_map: Arc<DashMap<u32, ButtplugClientDeviceInternal>>,
    span: Span) 
    -> Result<Self, ButtplugClientError> {
    // Create the client
    let mut client = ButtplugClient {
      client_name: client_name.to_string(),
      server_name: String::new(),
      message_sender,
      // Since we'll have already connected and initialized by the time we hand
      // this to the client function, we can go ahead and declare that we're
      // connected here. If that's not true, we won't even execute the client
      // function.
      connected: connected_status,
      device_map,
      _client_span: span
    };

    // Run our handshake
    info!("Running handshake with server.");
    match client
    .send_message(
      RequestServerInfo::new(&client.client_name, ButtplugMessageSpecVersion::Version2).into(),
    )
    .await {
      Ok(msg) => {
        debug!("Got ServerInfo return.");
        if let ButtplugCurrentSpecServerMessage::ServerInfo(server_info) = msg {
          info!("Connected to {}", server_info.server_name);
          client.server_name = server_info.server_name;
          // TODO Handle ping time in the internal event loop

          // Get currently connected devices. The event loop will
          // handle sending the message and getting the return, and
          // will send the client updates as events.
          let msg = client
            .send_message(RequestDeviceList::default().into())
            .await?;
          if let ButtplugCurrentSpecServerMessage::DeviceList(m) = msg {
            client
              .send_internal_message(ButtplugClientRequest::HandleDeviceList(m))
              .await?;
          }
          Ok(client)
        } else {
          client.disconnect().await?;
          Err(ButtplugClientError::ButtplugError(
            ButtplugError::ButtplugHandshakeError(ButtplugHandshakeError {
              message: "Did not receive expected ServerInfo or Error messages.".to_string(),
            }),
          ))
        }
      }
      // TODO Error message case may need to be implemented here when
      // we aren't only using embedded connectors.
      Err(e) => Err(e.into()),
    }
  }

  /// Returns true if client is currently connected.
  pub fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  /// Disconnects from server, if connected.
  ///
  /// Returns Err(ButtplugClientError) if disconnection fails. It can be assumed
  /// that even on failure, the client will be disconnected.
  pub fn disconnect(&self) -> ButtplugClientResultFuture {
    // Send the connector to the internal loop for management. Once we throw
    // the connector over, the internal loop will handle connecting and any
    // further communications with the server, if connection is successful.
    let fut = ButtplugConnectorFuture::default();
    let msg = ButtplugClientRequest::Disconnect(fut.get_state_clone());
    let send_fut = self.send_internal_message(msg);
    let connected = self.connected.clone();
    Box::pin(async move {
      send_fut.await?;
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
  }

  /// Tells server to start scanning for devices.
  ///
  /// Returns Err([ButtplugClientError]) if request fails due to issues with
  /// DeviceManagers on the server, disconnection, etc.
  pub fn start_scanning(&self) -> ButtplugClientResultFuture {
    self.send_message_expect_ok(StartScanning::default().into())
  }

  /// Send message to the internal event loop. 
  ///
  /// Mostly for handling boilerplate around possible send errors.
  fn send_internal_message(&self, msg: ButtplugClientRequest) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    if !self.connected.load(Ordering::SeqCst) {
      return Box::pin(future::ready(Err(
        ButtplugConnectorError::new("Client not connected")
      )));
    }
    // If we're running the event loop, we should have a message_sender.
    // Being connected to the server doesn't matter here yet because we use
    // this function in order to connect also.
    let message_sender = self.message_sender.clone();
    Box::pin(async move {
      message_sender.send(msg).await.map_err(|err| ButtplugConnectorError::new(&format!("Error with connector channel: {}", err)))?;
      Ok(())
    })
  }

  /// Sends a ButtplugMessage from client to server. Expects to receive a
  /// ButtplugMessage back from the server.
  fn send_message(
    &self,
    msg: ButtplugCurrentSpecClientMessage,
  ) -> ButtplugInternalClientMessageResultFuture {
    // Create a future to pair with the message being resolved.
    let fut = ButtplugClientMessageFuture::default();
    let internal_msg = ButtplugClientRequest::Message(ButtplugClientMessageFuturePair::new(
      msg,
      fut.get_state_clone(),
    ));

    // Send message to internal loop and wait for return.
    let send_fut = self.send_internal_message(internal_msg);
    Box::pin(async move {
      send_fut.await?;
      fut.await
    })
  }

  /// Sends a ButtplugMessage from client to server. Expects to receive an [Ok]
  /// type ButtplugMessage back from the server.
  fn send_message_expect_ok(
    &self,
    msg: ButtplugCurrentSpecClientMessage,
  ) -> ButtplugClientResultFuture {
    let send_fut = self.send_message(msg);
    Box::pin(async move {
      send_fut
        .await
        .and_then(|_| Ok(()))
        .map_err(|err| ButtplugMessageError::new(&format!("Got non-Ok message back: {:?}", err)).into())
    })
  }

  /// Retreives a list of currently connected devices.
  ///
  /// As the device list is maintained in the event loop structure, retreiving
  /// the list requires an asynchronous call to retreive the list from the task.
  pub fn devices(&self) -> Vec<ButtplugClientDevice> {
    info!("Request devices from inner loop!");
    let mut device_clones = vec!();
    for device in self.device_map.iter() {
      device_clones.push(ButtplugClientDevice::from((&(*device.device), self.message_sender.clone(), (*device.channel).clone())));
    }
    device_clones
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use super::ButtplugClient;
  use crate::{
    connector::{
      ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture,
      ButtplugInProcessClientConnector,
    },
    core::messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
    util::async_manager
  };
  use async_channel::Receiver;
  use futures::future::BoxFuture;

  #[derive(Default)]
  struct ButtplugFailingConnector {}

  impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
    for ButtplugFailingConnector
  {
    fn connect(
      &mut self,
    ) -> BoxFuture<
      'static,
      Result<Receiver<ButtplugCurrentSpecServerMessage>, ButtplugConnectorError>,
    > {
      ButtplugConnectorError::new("Always fails").into()
    }

    fn disconnect(&self) -> ButtplugConnectorResultFuture {
      ButtplugConnectorError::new("Always fails").into()
    }

    fn send(&self, _msg: ButtplugCurrentSpecClientMessage) -> ButtplugConnectorResultFuture {
      panic!("Should never be called")
    }
  }

  #[test]
  fn test_failing_connection() {
    async_manager::block_on(async {
      assert!(
        ButtplugClient::connect("Test Client", ButtplugFailingConnector::default())
        .await
        .is_err()
      );
    });
  }

  #[test]
  fn test_disconnect_status() {
    async_manager::block_on(async {
      let (client, _) = ButtplugClient::connect(
        "Test Client",
        ButtplugInProcessClientConnector::new("Test Server", 0),
      )
      .await.unwrap();
      assert!(client.disconnect().await.is_ok());
      assert!(!client.connected());
    });
  }

  #[test]
  fn test_double_disconnect() {
    async_manager::block_on(async {
      let (client, _) = ButtplugClient::connect(
        "Test Client",
        ButtplugInProcessClientConnector::new("Test Server", 0),
      )
      .await.unwrap();
      assert!(client.disconnect().await.is_ok());
      assert!(client.disconnect().await.is_err());
    });
  }

  #[test]
  fn test_connect_init() {
    async_manager::block_on(async {
      let (client, _) = ButtplugClient::connect(
        "Test Client",
        ButtplugInProcessClientConnector::new("Test Server", 0),
      )
      .await.unwrap();
      assert_eq!(client.server_name, "Test Server");
    });
  }

  // Test ignored until we have a test device manager.
  #[test]
  #[ignore]
  fn test_start_scanning() {
    async_manager::block_on(async {
      let (client, _) = ButtplugClient::connect(
        "Test Client",
        ButtplugInProcessClientConnector::new("Test Server", 0),
      )
      .await.unwrap();
      assert!(client.start_scanning().await.is_ok());
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
