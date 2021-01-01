// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use crate::{
  connector::{
    transport::{
      ButtplugConnectorTransport,
      ButtplugConnectorTransportConnectResult,
      ButtplugConnectorTransportSpecificError,
      ButtplugTransportIncomingMessage,
      ButtplugTransportOutgoingMessage,
    },
    ButtplugConnectorError,
    ButtplugConnectorResultFuture,
  },
  core::messages::serializer::ButtplugSerializedMessage,
  util::async_manager,
};
use async_channel::{bounded, Sender};
use async_lock::Mutex;
use async_tungstenite::{
  async_std::connect_async_with_tls_connector,
  tungstenite::protocol::Message,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugWebsocketClientTransport {
  /// Address of the server we'll connect to.
  address: String,
  /// If true, use a TLS wrapper on our connection.
  should_use_tls: bool,
  /// If true, bypass certificate verification. Should be true for self-signed
  /// certs.
  bypass_cert_verify: bool,
  /// Internally held sender, used for when disconnect is called.
  disconnect_sender: Arc<Mutex<Sender<ButtplugTransportOutgoingMessage>>>,
}

impl ButtplugWebsocketClientTransport {
  fn create(address: &str, should_use_tls: bool, bypass_cert_verify: bool) -> Self {
    // Create a dummy channel here. Saves us having to check option on every
    // call, and if it fails it means we're disconnected anyways, which is fine.
    let (unused_sender, _) = bounded(256);
    Self {
      should_use_tls,
      address: address.to_owned(),
      bypass_cert_verify,
      disconnect_sender: Arc::new(Mutex::new(unused_sender)),
    }
  }

  /// Creates a new connector for "ws://" addresses
  ///
  /// Returns a websocket connector for connecting over insecure websockets to a
  /// server. Address should be the full URL of the server, i.e.
  /// "ws://127.0.0.1:12345"
  pub fn new_insecure_connector(address: &str) -> Self {
    ButtplugWebsocketClientTransport::create(address, false, false)
  }

  /// Creates a new connector for "wss://" addresses
  ///
  /// Returns a websocket connector for connecting over secure websockets to a
  /// server. Address should be the full URL of the server, i.e.
  /// "ws://127.0.0.1:12345". If `bypass_cert_verify` is true, then the
  /// certificate of the server will not be verified (useful for servers using
  /// self-signed certs).
  pub fn new_secure_connector(address: &str, bypass_cert_verify: bool) -> Self {
    ButtplugWebsocketClientTransport::create(address, true, bypass_cert_verify)
  }
}

impl ButtplugConnectorTransport for ButtplugWebsocketClientTransport {
  fn connect(&self) -> ButtplugConnectorTransportConnectResult {
    let (request_sender, request_receiver) = bounded(256);
    let (response_sender, response_receiver) = bounded(256);

    let disconnect_sender_clone = self.disconnect_sender.clone();

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
    let address = self.address.clone();

    Box::pin(async move {
      *disconnect_sender_clone.lock().await = request_sender.clone();
      match connect_async_with_tls_connector(&address, tls_connector).await {
        Ok((stream, _)) => {
          let (mut writer, mut reader) = stream.split();
          // TODO Do we want to store/join these tasks anywhere?
          async_manager::spawn(async move {
            while let Ok(msg) = request_receiver.recv().await {
              let out_msg = match msg {
                ButtplugTransportOutgoingMessage::Message(outgoing_msg) => match outgoing_msg {
                  ButtplugSerializedMessage::Text(text) => Message::Text(text),
                  ButtplugSerializedMessage::Binary(bin) => Message::Binary(bin),
                },
                ButtplugTransportOutgoingMessage::Close => {
                  // If we can't close, just print the error to the logs but
                  // still break out of the loop.
                  //
                  // TODO Emit a full error here that should bubble up to the client.
                  writer.close().await.unwrap_or_else(|err| error!("{}", err));
                  return;
                }
              };
              // TODO see what happens when we try to send to a remote that's closed connection.
              writer.send(out_msg).await.expect("This should never fail?");
            }
          })
          .unwrap();
          async_manager::spawn(async move {
            while let Some(response) = reader.next().await {
              trace!("Websocket receiving: {:?}", response);
              match response {
                Ok(msg) => match msg {
                  Message::Text(t) => {
                    if response_sender
                      .send(ButtplugTransportIncomingMessage::Message(
                        ButtplugSerializedMessage::Text(t.to_string()),
                      ))
                      .await
                      .is_err()
                    {
                      error!("Websocket holder has closed, exiting websocket loop.");
                      return;
                    }
                  }
                  Message::Binary(v) => {
                    if response_sender
                      .send(ButtplugTransportIncomingMessage::Message(
                        ButtplugSerializedMessage::Binary(v),
                      ))
                      .await
                      .is_err()
                    {
                      error!("Websocket holder has closed, exiting websocket loop.");
                      return;
                    }
                  }
                  Message::Ping(_) => {}
                  Message::Pong(_) => {}
                  Message::Close(_) => {
                    info!("Websocket has requested close.");
                    return;
                  }
                },
                Err(err) => {
                  error!(
                    "Error in websocket client loop (assuming disconnect): {}",
                    err
                  );
                  break;
                }
              }
            }
          })
          .unwrap();
          Ok((request_sender, response_receiver))
        }
        Err(websocket_error) => Err(ButtplugConnectorError::TransportSpecificError(
          ButtplugConnectorTransportSpecificError::TungsteniteError(websocket_error),
        )),
      }
    })
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_sender = self.disconnect_sender;
    Box::pin(async move {
      // If we can't send the message, we have no loop, so we're not connected.
      if disconnect_sender
        .lock()
        .await
        .send(ButtplugTransportOutgoingMessage::Close)
        .await
        .is_err()
      {
        Err(ButtplugConnectorError::ConnectorNotConnected)
      } else {
        Ok(())
      }
    })
  }
}
