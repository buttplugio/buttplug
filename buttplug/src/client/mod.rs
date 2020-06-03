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
use internal::{client_event_loop, ButtplugClientRequest};

use crate::{
  connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorFuture},
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugHandshakeError, ButtplugMessageError},
    messages::{
      ButtplugCurrentSpecClientMessage,
      ButtplugCurrentSpecServerMessage,
      ButtplugMessageSpecVersion,
      DeviceMessageInfo,
      LogLevel,
      RequestDeviceList,
      RequestServerInfo,
      StartScanning,
    },
  },
  util::future::{ButtplugFuture, ButtplugFutureStateShared},
};

use async_std::{
  prelude::FutureExt,
  sync::{channel, Receiver, Sender},
};
use futures::{Future, StreamExt};
use std::{error::Error, fmt};

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

/// Result type used for passing server responses.
pub type ButtplugInternalClientMessageResult =
  ButtplugInternalClientResult<ButtplugCurrentSpecServerMessage>;
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
  message_sender: Sender<ButtplugClientRequest>,
  // Receives event notifications from the ButtplugClientLoop
  event_receiver: Receiver<ButtplugClientEvent>,
  // True if the connector is currently connected, and handshake was
  // successful.
  connected: bool,
  // Storage for events received when checking for events during
  // non-wait_for_event calls.
  events: Vec<ButtplugClientEvent>,
}

unsafe impl Sync for ButtplugClient {
}
unsafe impl Send for ButtplugClient {
}

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
  /// Will return Err([ButtplugClientError]) if connection with the server fails.
  ///
  /// # Examples
  ///
  /// ```
  /// #[cfg(feature = "server")]
  /// use buttplug::{
  ///   client::ButtplugClient,
  ///   connector::ButtplugInProcessClientConnector
  /// };
  ///
  /// #[cfg(feature = "server")]
  /// futures::executor::block_on(async {
  ///     ButtplugClient::run("Test Client", ButtplugInProcessClientConnector::new("Test Server", 0), |mut client| {
  ///         async move {
  ///             println!("Are we connected? {}", client.connected());
  ///         }
  ///     }).await;
  /// });
  /// ```
  pub async fn run<ApplicationFunctionType, ApplicationFunctionReturnType>(
    name: &str,
    mut connector: impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
      + 'static,
    func: ApplicationFunctionType,
  ) -> ButtplugClientResult
  where
    ApplicationFunctionType: FnOnce(ButtplugClient) -> ApplicationFunctionReturnType,
    ApplicationFunctionReturnType: Future<Output = ()>,
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

    let connector_receiver = connector.connect().await.map_err(|e| {
      let err: ButtplugClientError = e.into();
      err
    })?;

    let app_future = async move {
      client.handshake().await?;
      func(client).await;
      Ok(())
    };

    let internal_loop_future = client_event_loop(
      connector,
      connector_receiver,
      event_sender,
      message_receiver,
    );
    app_future.race(internal_loop_future).await
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
  /// # Panics
  ///
  /// If the library was compiled without any device managers, the
  /// [ButtplugClient] will have nothing to do. This is considered a
  /// catastrophic failure and the library will panic.
  ///
  /// If the library is using outside device managers, it is recommended to
  /// build your own connector, add your device manager to those, and use the
  /// `run()` method to pass it in.
  pub async fn run_with_in_process_connector<F, T>(
    name: &str,
    max_ping_time: u64,
    func: F,
  ) -> ButtplugClientResult
  where
    F: FnOnce(ButtplugClient) -> T,
    T: Future<Output = ()>,
  {
    #[cfg(not(any(feature = "btleplug-manager", feature = "xinput")))]
    panic!("Must compile library using at least one device communication manager features (btleplug, xinput, etc) to use run_with_in_process_connector.");
    #[cfg(any(feature = "btleplug-manager", feature = "xinput"))]
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
    ButtplugClient::run(name, connector, func).await
  }

  // Runs the handshake flow with the server.
  //
  // Sends over RequestServerInfo, gets back ServerInfo, sets up ping timer if
  // needed.
  async fn handshake(&mut self) -> ButtplugClientResult {
    info!("Running handshake with server.");
    match self
      .send_message(
        &RequestServerInfo::new(&self.client_name, ButtplugMessageSpecVersion::Version2).into(),
      )
      .await
    {
      Ok(msg) => {
        debug!("Got ServerInfo return.");
        if let ButtplugCurrentSpecServerMessage::ServerInfo(server_info) = msg {
          info!("Connected to {}", server_info.server_name);
          self.server_name = Option::Some(server_info.server_name);
          // TODO Handle ping time in the internal event loop

          // Get currently connected devices. The event loop will
          // handle sending the message and getting the return, and
          // will send the client updates as events.
          let msg = self
            .send_message(&RequestDeviceList::default().into())
            .await?;
          if let ButtplugCurrentSpecServerMessage::DeviceList(m) = msg {
            self
              .send_internal_message(ButtplugClientRequest::HandleDeviceList(m))
              .await?;
          }
          Ok(())
        } else {
          self.disconnect().await?;
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
    self.connected
  }

  /// Disconnects from server, if connected.
  ///
  /// Returns Err(ButtplugClientError) if disconnection fails. It can be assumed
  /// that even on failure, the client will be disconnected.
  pub async fn disconnect(&mut self) -> ButtplugClientResult {
    // Send the connector to the internal loop for management. Once we throw
    // the connector over, the internal loop will handle connecting and any
    // further communications with the server, if connection is successful.
    let fut = ButtplugConnectorFuture::default();
    let msg = ButtplugClientRequest::Disconnect(fut.get_state_clone());
    self.send_internal_message(msg).await?;
    self.connected = false;
    Ok(())
  }

  /// Tells server to start scanning for devices.
  ///
  /// Returns Err([ButtplugClientError]) if request fails due to issues with
  /// DeviceManagers on the server, disconnection, etc.
  pub async fn start_scanning(&mut self) -> ButtplugClientResult {
    self
      .send_message_expect_ok(&StartScanning::default().into())
      .await
  }

  // Send message to the internal event loop. Mostly for handling boilerplate
  // around possible send errors.
  async fn send_internal_message(
    &mut self,
    msg: ButtplugClientRequest,
  ) -> Result<(), ButtplugConnectorError> {
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
    msg: &ButtplugCurrentSpecClientMessage,
  ) -> ButtplugInternalClientMessageResult {
    // Create a future to pair with the message being resolved.
    let fut = ButtplugClientMessageFuture::default();
    let internal_msg = ButtplugClientRequest::Message(ButtplugClientMessageFuturePair::new(
      msg.clone(),
      fut.get_state_clone(),
    ));

    // Send message to internal loop and wait for return.
    self.send_internal_message(internal_msg).await?;
    fut.await
  }

  // Sends a ButtplugMessage from client to server. Expects to receive an [Ok]
  // type ButtplugMessage back from the server.
  async fn send_message_expect_ok(
    &mut self,
    msg: &ButtplugCurrentSpecClientMessage,
  ) -> ButtplugClientResult {
    match self.send_message(msg).await? {
      ButtplugCurrentSpecServerMessage::Ok(_) => Ok(()),
      _ => Err(ButtplugMessageError::new("Got non-Ok message back").into()),
    }
  }

  async fn check_for_events(&mut self) -> Result<(), ButtplugConnectorError> {
    if !self.connected {
      return Err(ButtplugConnectorError::new("Client not connected."));
    }
    while !self.event_receiver.is_empty() {
      match self.event_receiver.next().await {
        Some(msg) => self.events.push(msg),
        None => {
          self.connected = false;
          // If we got None, this means the internal loop stopped and our
          // sender was dropped. We should consider this a disconnect.
          self.events.push(ButtplugClientEvent::ServerDisconnect);
          return Err(ButtplugConnectorError::new("Client not connected."));
        }
      }
    }
    Ok(())
  }

  /// Produces a future that will wait for events from the internal loop.
  ///
  /// This should be called whenever the client isn't doing anything otherwise,
  /// so we can respond to unexpected updates from the server, such as devices
  /// connections/disconnections, log messages, etc... This is basically what
  /// event handlers in C# and JS would deal with, but we're in Rust so this
  /// requires us to be slightly more explicit. It will return
  /// Err([ButtplugConnectorError]) if waiting fails due to server/client
  /// disconnection.
  pub async fn wait_for_event(&mut self) -> Result<ButtplugClientEvent, ButtplugConnectorError> {
    debug!("Client waiting for event.");
    if !self.connected {
      return Err(ButtplugConnectorError::new("Client not connected."));
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

  /// Retreives a list of currently connected devices.
  ///
  /// As the device list is maintained in the event loop structure, retreiving
  /// the list requires an asynchronous call to retreive the list from the task.
  pub async fn devices(&mut self) -> Result<Vec<ButtplugClientDevice>, ButtplugConnectorError> {
    info!("Request devices from inner loop!");
    let fut = ButtplugFuture::<Vec<ButtplugClientDevice>>::default();
    let msg = ButtplugClientRequest::RequestDeviceList(fut.get_state_clone());
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
    connector::{
      ButtplugConnector,
      ButtplugConnectorError,
      ButtplugConnectorResultFuture,
      ButtplugInProcessClientConnector,
    },
    core::messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
  };
  use futures::future::BoxFuture;
  use async_std::{future::Future, sync::Receiver, task};

  async fn connect_test_client<F, T>(func: F)
  where
    F: FnOnce(ButtplugClient) -> T,
    T: Future<Output = ()>,
  {
    let _ = env_logger::builder().is_test(true).try_init();
    assert!(ButtplugClient::run(
      "Test Client",
      ButtplugInProcessClientConnector::new("Test Server", 0),
      func
    )
    .await
    .is_ok());
  }

  #[derive(Default)]
  struct ButtplugFailingConnector {}

  impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
    for ButtplugFailingConnector
  {
    fn connect(
      &mut self,
    ) -> BoxFuture<'static, Result<Receiver<ButtplugCurrentSpecServerMessage>, ButtplugConnectorError>> {
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
    let _ = env_logger::builder().is_test(true).try_init();
    task::block_on(async {
      assert!(
        ButtplugClient::run("Test Client", ButtplugFailingConnector::default(), |_| {
          async {}
        })
        .await
        .is_err()
      );
    });
  }

  #[test]
  fn test_disconnect_status() {
    task::block_on(async {
      connect_test_client(|mut client| async move {
        assert!(client.disconnect().await.is_ok());
        assert!(!client.connected());
      })
      .await;
    });
  }

  #[test]
  fn test_double_disconnect() {
    task::block_on(async {
      connect_test_client(|mut client| async move {
        assert!(client.disconnect().await.is_ok());
        assert!(client.disconnect().await.is_err());
      })
      .await;
    });
  }

  #[test]
  fn test_connect_init() {
    task::block_on(async {
      connect_test_client(|client| async move {
        assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
      })
      .await;
    });
  }

  // Test ignored until we have a test device manager.
  #[test]
  #[ignore]
  fn test_start_scanning() {
    task::block_on(async {
      connect_test_client(|mut client| async move {
        assert!(client.start_scanning().await.is_ok());
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
