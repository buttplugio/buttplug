use crate::{
  connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResult},
  core::messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
  server::ButtplugServer,
};
use async_std::{
  prelude::StreamExt,
  sync::{channel, Receiver, Sender},
  task,
};
use async_trait::async_trait;
use std::convert::TryInto;

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
  server: ButtplugServer,
  server_outbound_sender: Sender<ButtplugCurrentSpecServerMessage>,
  /// Event receiver for the internal server.
  connector_outbound_recv: Option<Receiver<ButtplugCurrentSpecServerMessage>>,
}

#[cfg(feature = "server")]
impl<'a> ButtplugInProcessClientConnector {
  /// Creates a new in-process connector, with a server instance.
  ///
  /// Sets up a server, using the basic [ButtplugServer] construction arguments.
  /// Takes the server's name and the ping time it should use, with a ping time
  /// of 0 meaning infinite ping.
  pub fn new(name: &str, max_ping_time: u64) -> Self {
    let (server, mut server_recv) = ButtplugServer::new(&name, max_ping_time);
    let (send, recv) = channel(256);
    let server_outbound_sender = send.clone();
    task::spawn(async move {
      while let Some(event) = server_recv.next().await {
        send.send(event.try_into().unwrap()).await;
      }
    });

    Self {
      connector_outbound_recv: Some(recv),
      server_outbound_sender,
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
    &mut self.server
  }
}

#[cfg(feature = "server")]
#[async_trait]
impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
  for ButtplugInProcessClientConnector
{
  async fn connect(
    &mut self,
  ) -> Result<Receiver<ButtplugCurrentSpecServerMessage>, ButtplugConnectorError> {
    if self.connector_outbound_recv.is_some() {
      Ok(self.connector_outbound_recv.take().unwrap())
    } else {
      Err(ButtplugConnectorError::new("Connector already connected."))
    }
  }

  async fn disconnect(&mut self) -> ButtplugConnectorResult {
    Ok(())
  }

  async fn send(&mut self, msg: ButtplugCurrentSpecClientMessage) -> ButtplugConnectorResult {
    let input = msg.try_into().unwrap();
    let output = self.server.parse_message(input).await.unwrap();
    self
      .server_outbound_sender
      .send(output.try_into().unwrap())
      .await;
    Ok(())
  }
}
