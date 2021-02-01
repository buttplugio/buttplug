use crate::{
  connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture},
  core::{
    errors::ButtplugError,
    messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
  },
  server::{ButtplugServer, ButtplugServerOptions},
  util::async_manager,
};
use futures::{
  future::{self, BoxFuture},
  StreamExt,
};
use std::{
  convert::TryInto,
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::mpsc::{channel, Sender};
use tracing_futures::Instrument;

/// In-process Buttplug Server Connector
///
/// The In-Process Connector contains a [ButtplugServer], meaning that both the
/// [ButtplugClient][crate::client::ButtplugClient] and [ButtplugServer] will
/// exist in the same process. This is useful for developing applications, or
/// for distributing an applications without requiring access to an outside
/// [ButtplugServer].
///
/// # Notes
///
/// Buttplug, as a project, is built in a way that tries to make sure all
/// programs will work with new versions of the library. This is why we have
/// [ButtplugClient][crate::client::ButtplugClient] for applications, and
/// Connectors to access out-of-process [ButtplugServer]s over IPC, network,
/// etc. It means that the out-of-process server can be upgraded by the user at
/// any time, even if the [ButtplugClient][crate::client::ButtplugClient] using
/// application hasn't been upgraded. This allows the program to support
/// hardware that may not have even been released when it was published.
///
/// While including an EmbeddedConnector in your application is the quickest and
/// easiest way to develop (and we highly recommend developing that way), and
/// also an easy way to get users up and running as quickly as possible, we
/// recommend also including some sort of IPC Connector in order for your
/// application to connect to newer servers when they come out.
#[cfg(feature = "server")]
pub struct ButtplugInProcessClientConnector {
  /// Internal server object for the embedded connector.
  server: Arc<ButtplugServer>,
  server_outbound_sender: Sender<ButtplugCurrentSpecServerMessage>,
  connected: Arc<AtomicBool>,
}

#[cfg(feature = "server")]
impl<'a> Default for ButtplugInProcessClientConnector {
  fn default() -> Self {
    // Unwrap is fine here, if we pass in default options we'll never fail.
    ButtplugInProcessClientConnector::new_with_options(&ButtplugServerOptions::default()).unwrap()
  }
}

#[cfg(feature = "server")]
impl<'a> ButtplugInProcessClientConnector {
  /// Creates a new in-process connector, with a server instance.
  ///
  /// Sets up a server, using the basic [ButtplugServer] construction arguments.
  /// Takes the server's name and the ping time it should use, with a ping time
  /// of 0 meaning infinite ping.
  pub fn new_with_options(options: &ButtplugServerOptions) -> Result<Self, ButtplugError> {
    let server = Arc::new(ButtplugServer::new_with_options(options)?);
    // Create a dummy channel, will just be overwritten on connect.
    let (server_outbound_sender, _) = channel(256);
    Ok(Self {
      server_outbound_sender,
      server,
      connected: Arc::new(AtomicBool::new(false)),
    })
  }

  /// Get a reference to the internal server.
  ///
  /// Allows the owner to manipulate the internal server instance. Useful for
  /// setting up
  /// [DeviceCommunicationManager][crate::server::comm_managers::DeviceCommunicationManager]s
  /// before connection.
  pub fn server_ref(&'a self) -> &'a ButtplugServer {
    &self.server
  }
}

#[cfg(feature = "server")]
impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
  for ButtplugInProcessClientConnector
{
  fn connect(
    &mut self,
    message_sender: Sender<ButtplugCurrentSpecServerMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    if !self.connected.load(Ordering::SeqCst) {
      let connected = self.connected.clone();
      let send = message_sender.clone();
      self.server_outbound_sender = message_sender;
      let server_recv = self.server.event_stream();
      Box::pin(async move {
        async_manager::spawn(async move {
          info!("Starting In Process Client Connector Event Sender Loop");
          pin_mut!(server_recv);
          while let Some(event) = server_recv.next().await {
            // If we get an error back, it means the client dropped our event
            // handler, so just stop trying. Otherwise, since this is an
            // in-process conversion, we can unwrap because we know our
            // try_into() will always succeed (which may not be the case with
            // remote connections that have different spec versions).
            if send.send(event.try_into().unwrap()).await.is_err() {
              break;
            }
          }
          info!("Stopping In Process Client Connector Event Sender Loop, due to channel receiver being dropped.");
        }.instrument(tracing::info_span!("InProcessClientConnectorEventSenderLoop"))).unwrap();
        connected.store(true, Ordering::SeqCst);
        Ok(())
      })
    } else {
      ButtplugConnectorError::ConnectorAlreadyConnected.into()
    }
  }

  fn disconnect(&self) -> ButtplugConnectorResultFuture {
    if self.connected.load(Ordering::SeqCst) {
      self.connected.store(false, Ordering::SeqCst);
      Box::pin(future::ready(Ok(())))
    } else {
      ButtplugConnectorError::ConnectorNotConnected.into()
    }
  }

  fn send(&self, msg: ButtplugCurrentSpecClientMessage) -> ButtplugConnectorResultFuture {
    if !self.connected.load(Ordering::SeqCst) {
      return ButtplugConnectorError::ConnectorNotConnected.into();
    }
    let input = msg.try_into().unwrap();
    let output_fut = self.server.parse_message(input);
    let sender = self.server_outbound_sender.clone();
    Box::pin(async move {
      // Once again, this is an in process server, so we know we'll always be
      // running on the same spec version. Therefore we can just unwrap after
      // the try_into() conversion.
      let output: ButtplugCurrentSpecServerMessage = output_fut
        .await
        .unwrap_or_else(|e| e.into())
        .try_into()
        .unwrap();
      sender
        .send(output)
        .await
        .map_err(|_| ButtplugConnectorError::ConnectorNotConnected)
    })
  }
}
