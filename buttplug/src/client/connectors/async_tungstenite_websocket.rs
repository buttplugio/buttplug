// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use super::{
  ButtplugClientConnector, ButtplugClientConnectorError, ButtplugClientConnectorResult,
  ButtplugRemoteClientConnectorHelper, ButtplugRemoteClientConnectorMessage,
};
use crate::{
  client::ButtplugInternalClientMessageResult,
  core::messages::{ButtplugClientInMessage, ButtplugClientOutMessage, ButtplugMessage},
};
use async_std::{
  sync::{channel, Receiver},
  task,
};
use async_trait::async_trait;
use async_tungstenite::{
  async_std::connect_async_with_tls_connector, tungstenite::protocol::Message,
};
use futures_util::{SinkExt, StreamExt};

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct AsyncTungsteniteWebsocketClientConnector {
  /// Remote connector helper, for setting message indexes and resolving futures
  helper: ButtplugRemoteClientConnectorHelper,
  /// Receiver of messages from the server, for sending to the client.
  recv: Option<Receiver<ButtplugClientOutMessage>>,
  /// Address of the server we'll connect to.
  address: String,
  /// If true, use a TLS wrapper on our connection.
  should_use_tls: bool,
  /// If true, bypass certificate verification. Should be true for self-signed
  /// certs.
  bypass_cert_verify: bool,
}

impl AsyncTungsteniteWebsocketClientConnector {
  /// Creates a new connector for "ws://" addresses
  ///
  /// Returns a websocket connector for connecting over insecure websockets to a
  /// server. Address should be the full URL of the server, i.e.
  /// "ws://127.0.0.1:12345"
  pub fn new_insecure_connector(address: &str) -> Self {
    Self {
      helper: ButtplugRemoteClientConnectorHelper::default(),
      recv: None,
      should_use_tls: false,
      address: address.to_owned(),
      bypass_cert_verify: false,
    }
  }

  /// Creates a new connector for "wss://" addresses
  ///
  /// Returns a websocket connector for connecting over secure websockets to a
  /// server. Address should be the full URL of the server, i.e.
  /// "ws://127.0.0.1:12345". If `bypass_cert_verify` is true, then the
  /// certificate of the server will not be verified (useful for servers using
  /// self-signed certs).
  pub fn new_secure_connector(address: &str, bypass_cert_verify: bool) -> Self {
    Self {
      helper: ButtplugRemoteClientConnectorHelper::default(),
      recv: None,
      should_use_tls: true,
      address: address.to_owned(),
      bypass_cert_verify,
    }
  }
}

#[async_trait]
impl ButtplugClientConnector for AsyncTungsteniteWebsocketClientConnector {
  async fn connect(&mut self) -> Result<(), ButtplugClientConnectorError> {
    let (client_sender, client_receiver) = channel(256);
    self.recv = Some(client_receiver);
    let (read_future, connector_input_recv, connector_output_sender) =
      self.helper.get_event_loop_future(client_sender);

    // If we're supposed to be a secure connection, generate a TLS connector
    // based on our certificate verfication needs. Otherwise, just pass None in
    // which case we won't wrap.
    let tls_connector = if self.should_use_tls {
      use async_tls::TlsConnector;
      if self.bypass_cert_verify {
        // If we need to connect to self signed cert using servers, we'll need
        // to create a validator that always passes. Got this one from
        // https://github.com/sdroege/async-tungstenite/issues/4#issuecomment-566923534
        use rustls::ClientConfig;
        use std::sync::Arc;

        pub struct NoCertificateVerification {}

        impl rustls::ServerCertVerifier for NoCertificateVerification {
          fn verify_server_cert(
            &self,
            _roots: &rustls::RootCertStore,
            _presented_certs: &[rustls::Certificate],
            _dns_name: webpki::DNSNameRef<'_>,
            _ocsp: &[u8],
          ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            Ok(rustls::ServerCertVerified::assertion())
          }
        }

        let mut config = ClientConfig::new();
        config
          .dangerous()
          .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
        Some(TlsConnector::from(Arc::new(config)))
      } else {
        Some(TlsConnector::new())
      }
    } else {
      // If we're not using a secure connection, just return None, at which
      // point async_tungstenite won't use a wrapper.
      None
    };

    match connect_async_with_tls_connector(&self.address, tls_connector).await {
      Ok((stream, _)) => {
        let (mut writer, mut reader) = stream.split();
        // TODO Do we want to store/join these tasks anywhere?
        task::spawn(async move {
          while let Some(msg) = connector_input_recv.recv().await {
            let json = msg.as_protocol_json();
            trace!("Websocket sending: {}", json);
            // TODO see what happens when we try to send to a remote that's closed connection.
            writer
              .send(Message::text(json))
              .await
              .expect("This should never fail?");
          }
        });
        task::spawn(async move {
          while let Some(response) = reader.next().await {
            trace!("Websocket receiving: {:?}", response);
            match response.unwrap() {
              Message::Text(t) => {
                connector_output_sender
                  .send(ButtplugRemoteClientConnectorMessage::Text(t.to_string()))
                  .await;
              }
              // TODO Do we need to handle anything else?
              Message::Binary(_) => {}
              Message::Ping(_) => {}
              Message::Pong(_) => {}
              Message::Close(_) => {}
            }
          }
        });
        task::spawn(async move {
          read_future.await;
        });
        Ok(())
      }
      Err(e) => Err(ButtplugClientConnectorError::new(&format!("{:?}", e))),
    }
  }

  async fn disconnect(&mut self) -> ButtplugClientConnectorResult {
    self.helper.close().await;
    Ok(())
  }

  async fn send(&mut self, msg: ButtplugClientInMessage) -> ButtplugInternalClientMessageResult {
    self.helper.send(&msg).await
  }

  fn get_event_receiver(&mut self) -> Receiver<ButtplugClientOutMessage> {
    // This will panic if we've already taken the receiver.
    self.recv.take().unwrap()
  }
}
