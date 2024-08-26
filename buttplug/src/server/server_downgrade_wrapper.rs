// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::{fmt, sync::Arc};

use crate::core::message::{self, ButtplugClientMessageVariant, ButtplugMessageSpecVersion, ButtplugServerMessageV4, ButtplugServerMessageVariant};

use super::{device::ServerDeviceManager, server_message_conversion::ButtplugServerMessageConverter, ButtplugServer, ButtplugServerResultFuture};
use futures::{future::{self, BoxFuture, FutureExt}, Stream};
use tokio_stream::StreamExt;
use once_cell::sync::OnceCell;

pub struct ButtplugServerDowngradeWrapper {
  /// Spec version of the currently connected client. Held as an atomic so we don't have to worry
  /// about locks when doing lookups.
  spec_version: Arc<OnceCell<ButtplugMessageSpecVersion>>,
  server: ButtplugServer,
}

impl std::fmt::Debug for ButtplugServerDowngradeWrapper {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.server.fmt(f)
  }
}


impl ButtplugServerDowngradeWrapper {
  pub fn new(server: ButtplugServer) -> Self {
    Self {
      spec_version: Arc::new(OnceCell::new()),
      server
    }
  }

  pub fn client_name(&self) -> Option<&String> {
    self.server.client_name()
  }

  /// Returns a references to the internal device manager, for handling configuration.
  pub fn device_manager(&self) -> Arc<ServerDeviceManager> {
    self.server.device_manager()
  }

  /// If true, client is currently connected to the server.
  pub fn connected(&self) -> bool {
    self.server.connected()
  }

  /// Disconnects the server from a client, if it is connected.
  pub fn disconnect(&self) -> BoxFuture<Result<(), message::ErrorV0>> {
    self.server.disconnect()
  }

  pub fn client_version_event_stream(&self) -> impl Stream<Item = ButtplugServerMessageVariant> {
    let spec_version = self.spec_version.clone();
    self.server.event_stream().map(move |m| {
        let converter = ButtplugServerMessageConverter::new(None);
        // If we get an event and don't have a spec version yet, just throw out the latest.
        converter.convert_outgoing(&m, spec_version.get().unwrap_or(&ButtplugMessageSpecVersion::Version4)).unwrap()
      }
    )
  }

  pub fn server_version_event_stream(&self) -> impl Stream<Item = ButtplugServerMessageV4> {
    // Unlike the client API, we can expect anyone using the server to pin this
    // themselves.
    self.server.event_stream()
  }

  /// Sends a [ButtplugClientMessage] to be parsed by the server (for handshake or ping), or passed
  /// into the server's [DeviceManager] for communication with devices.
  pub fn parse_message(
    &self,
    msg: ButtplugClientMessageVariant,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageVariant, message::ErrorV0>> {
    error!("{:?}", msg);
    match msg {
      ButtplugClientMessageVariant::V4(msg) => {
        let fut = self.server.parse_message(msg);
        async move {
          Ok(fut.await?.into())
        }.boxed()
      }
      msg => {
        let v = msg.version();
        let converter = ButtplugServerMessageConverter::new(Some(msg));
        match converter.convert_incoming(&self.server.device_manager()) {
          Ok(converted_msg) => {
            let fut = self.server.parse_message(converted_msg);
            let spec_version = *self.spec_version.get_or_init(|| {
              info!("Setting Buttplug Server Message Spec Downgrade version to {}", v);
              v
            } );
            async move {
              let result = fut.await?;
              converter.convert_outgoing(&result, &spec_version).map_err(|e| e.into())
            }.boxed()
          }
          Err(e) => future::ready(Err(e.into())).boxed()
        }
      }
    }
  }

  pub fn shutdown(&self) -> ButtplugServerResultFuture {
    self.server.shutdown()
  }

  pub fn destroy(self) -> ButtplugServer {
    self.server
  }
}