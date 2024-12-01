// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! In-process communication between clients and servers

use buttplug::{
  core::{
    connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture},
    errors::{ButtplugError, ButtplugMessageError},
  },
  server::{ButtplugServer, ButtplugServerBuilder, message::{ButtplugClientMessageV2, ButtplugServerMessageV2, ButtplugServerMessageVariant},
},
  util::async_manager,
};
use futures::{
  future::{self, BoxFuture, FutureExt},
  pin_mut,
  StreamExt,
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::mpsc::{channel, Sender};
use tracing::*;
use tracing_futures::Instrument;

#[derive(Default)]
pub struct ButtplugInProcessClientConnectorBuilder {
  server: Option<ButtplugServer>,
}

impl ButtplugInProcessClientConnectorBuilder {
  pub fn server(&mut self, server: ButtplugServer) -> &mut Self {
    self.server = Some(server);
    self
  }

  pub fn finish(&mut self) -> ButtplugInProcessClientConnector {
    ButtplugInProcessClientConnector::new(self.server.take())
  }
}

/// In-process Buttplug Server Connector
///
/// The In-Process Connector contains a [ButtplugServer], meaning that both the
/// [ButtplugClient][crate::client::ButtplugClient] and [ButtplugServer] will exist in the same
/// process. This is useful for developing applications, or for distributing an applications without
/// requiring access to an outside [ButtplugServer].
///
/// # Notes
///
/// Buttplug is built in a way that tries to make sure all programs will work with new versions of
/// the library. This is why we have [ButtplugClient][crate::client::ButtplugClient] for
/// applications, and Connectors to access out-of-process [ButtplugServer]s over IPC, network, etc.
/// It means that the out-of-process server can be upgraded by the user at any time, even if the
/// [ButtplugClient][crate::client::ButtplugClient] using application hasn't been upgraded. This
/// allows the program to support hardware that may not have even been released when it was
/// published.
///
/// While including an EmbeddedConnector in your application is the quickest and easiest way to
/// develop (and we highly recommend developing that way), and also an easy way to get users up and
/// running as quickly as possible, we recommend also including some sort of IPC Connector in order
/// for your application to connect to newer servers when they come out.
#[cfg(feature = "server")]
pub struct ButtplugInProcessClientConnector {
  /// Internal server object for the embedded connector.
  server: Arc<ButtplugServer>,
  server_outbound_sender: Sender<ButtplugServerMessageV2>,
  connected: Arc<AtomicBool>,
}

#[cfg(feature = "server")]
impl Default for ButtplugInProcessClientConnector {
  fn default() -> Self {
    ButtplugInProcessClientConnectorBuilder::default().finish()
  }
}

#[cfg(feature = "server")]
impl<'a> ButtplugInProcessClientConnector {
  /// Creates a new in-process connector, with a server instance.
  ///
  /// Sets up a server, using the basic [ButtplugServer] construction arguments.
  /// Takes the server's name and the ping time it should use, with a ping time
  /// of 0 meaning infinite ping.
  fn new(server: Option<ButtplugServer>) -> Self {
    // Create a dummy channel, will just be overwritten on connect.
    let (server_outbound_sender, _) = channel(256);
    Self {
      server_outbound_sender,
      server: Arc::new(server.unwrap_or_else(
        || {
          ButtplugServerBuilder::default()
            .finish()
            .expect("Default server builder should always work.")
        },
      )),
      connected: Arc::new(AtomicBool::new(false)),
    }
  }
}

#[cfg(feature = "server")]
impl ButtplugConnector<ButtplugClientMessageV2, ButtplugServerMessageV2>
  for ButtplugInProcessClientConnector
{
  fn connect(
    &mut self,
    message_sender: Sender<ButtplugServerMessageV2>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    if !self.connected.load(Ordering::SeqCst) {
      let connected = self.connected.clone();
      let send = message_sender.clone();
      self.server_outbound_sender = message_sender;
      let server_recv = self.server.event_stream();
      async move {
        async_manager::spawn(async move {
          info!("Starting In Process Client Connector Event Sender Loop");
          pin_mut!(server_recv);
          while let Some(event) = server_recv.next().await {
            // If we get an error back, it means the client dropped our event
            // handler, so just stop trying. Otherwise, since this is an
            // in-process conversion, we can unwrap because we know our
            // try_into() will always succeed (which may not be the case with
            // remote connections that have different spec versions).
            if let ButtplugServerMessageVariant::V2(msg) = event {
              if send.send(msg).await.is_err() {
                break;
              }
            } else {
              panic!("This is in-process so we're always on the latest message spec, this will always work.")
            }
          }
          info!("Stopping In Process Client Connector Event Sender Loop, due to channel receiver being dropped.");
        }.instrument(tracing::info_span!("InProcessClientConnectorEventSenderLoop")));
        connected.store(true, Ordering::SeqCst);
        Ok(())
      }.boxed()
    } else {
      ButtplugConnectorError::ConnectorAlreadyConnected.into()
    }
  }

  fn disconnect(&self) -> ButtplugConnectorResultFuture {
    if self.connected.load(Ordering::SeqCst) {
      self.connected.store(false, Ordering::SeqCst);
      future::ready(Ok(())).boxed()
    } else {
      ButtplugConnectorError::ConnectorNotConnected.into()
    }
  }

  fn send(&self, msg: ButtplugClientMessageV2) -> ButtplugConnectorResultFuture {
    if !self.connected.load(Ordering::SeqCst) {
      return ButtplugConnectorError::ConnectorNotConnected.into();
    }
    let input = msg.into();
    let output_fut = self.server.parse_message(input);
    let sender = self.server_outbound_sender.clone();
    async move {
      let output = match output_fut.await {
        Ok(m) => {
          if let ButtplugServerMessageVariant::V2(msg) = m {
            msg
          } else {
            ButtplugServerMessageV2::Error(
              ButtplugError::from(ButtplugMessageError::MessageConversionError(
                "In-process connector messages should never have differing versions.".to_owned(),
              ))
              .into(),
            )
          }
        }
        Err(e) => {
          if let ButtplugServerMessageVariant::V2(msg) = e {
            msg
          } else {
            ButtplugServerMessageV2::Error(
              ButtplugError::from(ButtplugMessageError::MessageConversionError(
                "In-process connector messages should never have differing versions.".to_owned(),
              ))
              .into(),
            )
          }
        }
      };
      sender
        .send(output)
        .await
        .map_err(|_| ButtplugConnectorError::ConnectorNotConnected)
    }
    .boxed()
  }
}
