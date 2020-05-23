use super::ButtplugInProcessServerConnector;
use crate::{
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage},
  server::ButtplugServer,
  connector::{ButtplugClientConnector, ButtplugServerConnector, ButtplugClientConnectorError}
};
use async_std::{sync::Receiver};
use async_trait::async_trait;

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
  server: ButtplugInProcessServerConnector,
  /// Event receiver for the internal server.
  recv: Option<Receiver<ButtplugClientOutMessage>>,
}

#[cfg(feature = "server")]
impl<'a> ButtplugInProcessClientConnector {
  /// Creates a new in-process connector, with a server instance.
  ///
  /// Sets up a server, using the basic [ButtplugServer] construction arguments.
  /// Takes the server's name and the ping time it should use, with a ping time
  /// of 0 meaning infinite ping.
  pub fn new(name: &str, max_ping_time: u128) -> Self {
    let (server, recv) = ButtplugInProcessServerConnector::new(&name, max_ping_time);
    Self {
      recv: Some(recv),
      server,
    }
  }

  /// Get a reference to the internal server.
  ///
  /// Allows the owner to manipulate the internal server instance. Useful for
  /// setting up
  /// [DeviceCommunicationManager][crate::server::comm_managers::DeviceCommunicationManager]s
  /// before connection.
  pub fn server_ref(&'a mut self) -> &'a mut ButtplugServer {
    self.server.server_ref()
  }
}

#[cfg(feature = "server")]
#[async_trait]
impl ButtplugClientConnector for ButtplugInProcessClientConnector {
  async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
    Ok(())
  }

  async fn disconnect(&mut self) -> Result<(), ButtplugClientConnectorError> {
    Ok(())
  }

  async fn send(
    &mut self,
    msg: ButtplugClientInMessage,
  ) -> Result<ButtplugClientOutMessage, ButtplugClientConnectorError> {
    Ok(self.server.parse_message(msg).await)
  }

  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage> {
    // This will panic if we've already taken the receiver.
    self.recv.take().unwrap()
  }
}

// The in-process connector is used heavily in the client unit tests, so we can
// assume code coverage there and omit specific tests here.
