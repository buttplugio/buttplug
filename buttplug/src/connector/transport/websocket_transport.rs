// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use crate::{
  connector::{
    ButtplugConnectorError, ButtplugConnectorResultFuture, ButtplugConnectorTransport,
    ButtplugTransportMessage,
  },
  core::messages::serializer::ButtplugSerializedMessage,
};
use async_std::{
  sync::{channel, Receiver, Sender},
  task,
};
use async_tungstenite::{
  async_std::connect_async_with_tls_connector, tungstenite::protocol::Message,
};
use futures::future::{self, BoxFuture};
use futures_util::{SinkExt, StreamExt};

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugWebsocketClientTransport {
  /// Address of the server we'll connect to.
  address: String,
  /// If true, use a TLS wrapper on our connection.
  should_use_tls: bool,
  /// If true, bypass certificate verification. Should be true for self-signed
  /// certs.
  bypass_cert_verify: bool,
}

impl ButtplugWebsocketClientTransport {
  /// Creates a new connector for "ws://" addresses
  ///
  /// Returns a websocket connector for connecting over insecure websockets to a
  /// server. Address should be the full URL of the server, i.e.
  /// "ws://127.0.0.1:12345"
  pub fn new_insecure_connector(address: &str) -> Self {
    Self {
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
      should_use_tls: true,
      address: address.to_owned(),
      bypass_cert_verify,
    }
  }
}

impl ButtplugConnectorTransport for ButtplugWebsocketClientTransport {
  fn connect(
    &self,
  ) -> BoxFuture<
    'static,
    Result<
      (
        Sender<ButtplugSerializedMessage>,
        Receiver<ButtplugTransportMessage>,
      ),
      ButtplugConnectorError,
    >,
  > {
    let (request_sender, request_receiver) = channel(256);
    let (response_sender, response_receiver) = channel(256);

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
    let address = self.address.clone();

    Box::pin(async move {
      match connect_async_with_tls_connector(&address, tls_connector).await {
        Ok((stream, _)) => {
          let (mut writer, mut reader) = stream.split();
          // TODO Do we want to store/join these tasks anywhere?
          task::spawn(async move {
            while let Some(msg) = request_receiver.recv().await {
              let out_msg = match msg {
                ButtplugSerializedMessage::Text(text) => Message::Text(text),
                ButtplugSerializedMessage::Binary(bin) => Message::Binary(bin),
              };
              // TODO see what happens when we try to send to a remote that's closed connection.
              writer.send(out_msg).await.expect("This should never fail?");
            }
          });
          task::spawn(async move {
            while let Some(response) = reader.next().await {
              trace!("Websocket receiving: {:?}", response);
              match response.unwrap() {
                Message::Text(t) => {
                  response_sender
                    .send(ButtplugTransportMessage::Message(
                      ButtplugSerializedMessage::Text(t.to_string()),
                    ))
                    .await;
                }
                // TODO Do we need to handle anything else?
                Message::Binary(v) => {
                  response_sender
                    .send(ButtplugTransportMessage::Message(
                      ButtplugSerializedMessage::Binary(v),
                    ))
                    .await;
                }
                Message::Ping(_) => {}
                Message::Pong(_) => {}
                Message::Close(_) => {}
              }
            }
          });
          Ok((request_sender, response_receiver))
        }
        Err(e) => Err(ButtplugConnectorError::new(&format!("{:?}", e))),
      }
    })
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    // TODO We should definitely allow people to disconnect. That would be a good thing.
    Box::pin(future::ready(Ok(())))
  }
}
