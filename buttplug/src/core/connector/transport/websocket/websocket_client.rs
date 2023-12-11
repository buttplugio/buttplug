// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Handling of websockets using async-tungstenite

use crate::{
  core::{
    connector::{
      transport::{
        ButtplugConnectorTransport,
        ButtplugConnectorTransportSpecificError,
        ButtplugTransportIncomingMessage,
      },
      ButtplugConnectorError,
      ButtplugConnectorResultFuture,
    },
    message::serializer::ButtplugSerializedMessage,
  },
  util::async_manager,
};
use futures::{future::BoxFuture, FutureExt, SinkExt, StreamExt};
use rustls::ClientConfig;
use std::sync::Arc;
use tokio::sync::{
  mpsc::{Receiver, Sender},
  Notify,
};
use tokio_tungstenite::{
  connect_async,
  connect_async_tls_with_config,
  tungstenite::protocol::Message,
  Connector,
};
use tracing::Instrument;
use url::Url;

// Taken from https://stackoverflow.com/questions/72846337/does-hyper-client-not-accept-self-signed-certificates
pub fn get_rustls_config_dangerous() -> ClientConfig {
  let store = rustls::RootCertStore::empty();

  let mut config = ClientConfig::builder()
    .with_safe_defaults()
    .with_root_certificates(store)
    .with_no_client_auth();

  // if you want to completely disable cert-verification, use this
  let mut dangerous_config = ClientConfig::dangerous(&mut config);
  dangerous_config.set_certificate_verifier(Arc::new(NoCertificateVerification {}));

  config
}
pub struct NoCertificateVerification {}
impl rustls::client::ServerCertVerifier for NoCertificateVerification {
  fn verify_server_cert(
    &self,
    _end_entity: &rustls::Certificate,
    _intermediates: &[rustls::Certificate],
    _server_name: &rustls::ServerName,
    _scts: &mut dyn Iterator<Item = &[u8]>,
    _ocsp: &[u8],
    _now: std::time::SystemTime,
  ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
    Ok(rustls::client::ServerCertVerified::assertion())
  }
}

/// Websocket connector for ButtplugClients, using [tokio_tungstenite]
pub struct ButtplugWebsocketClientTransport {
  /// Address of the server we'll connect to.
  address: String,
  /// If true, use a TLS wrapper on our connection.
  should_use_tls: bool,
  /// If true, bypass certificate verification. Should be true for self-signed
  /// certs.
  bypass_cert_verify: bool,
  /// Internally held sender, used for when disconnect is called.
  disconnect_notifier: Arc<Notify>,
}

impl ButtplugWebsocketClientTransport {
  fn create(address: &str, should_use_tls: bool, bypass_cert_verify: bool) -> Self {
    Self {
      should_use_tls,
      address: address.to_owned(),
      bypass_cert_verify,
      disconnect_notifier: Arc::new(Notify::new()),
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
  fn connect(
    &self,
    mut outgoing_receiver: Receiver<ButtplugSerializedMessage>,
    incoming_sender: Sender<ButtplugTransportIncomingMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    let disconnect_notifier = self.disconnect_notifier.clone();

    let address = self.address.clone();
    let should_use_tls = self.should_use_tls;
    let bypass_cert_verify = self.bypass_cert_verify;
    async move {
      let url = Url::parse(&address).expect("Should be checked before here");
      let stream_result = if should_use_tls {
        // If we're supposed to be a secure connection, generate a TLS connector
        // based on our certificate verfication needs. Otherwise, just pass None in
        // which case we won't wrap.
        let connector = if bypass_cert_verify {
          Some(Connector::Rustls(Arc::new(get_rustls_config_dangerous())))
        } else {
          None
        };
        connect_async_tls_with_config(&url, None, false, connector).await
      } else {
        connect_async(&url).await
      };

      match stream_result {
        Ok((stream, _)) => {
          let (mut writer, mut reader) = stream.split();

          async_manager::spawn(
            async move {
              loop {
                select! {
                  msg = outgoing_receiver.recv().fuse() => {
                    if let Some(msg) = msg {
                      let out_msg = match msg {
                        ButtplugSerializedMessage::Text(text) => Message::Text(text),
                        ButtplugSerializedMessage::Binary(bin) => Message::Binary(bin),
                      };
                      // TODO see what happens when we try to send to a remote that's closed connection.
                      writer.send(out_msg).await.expect("This should never fail?");
                    } else {
                      info!("Connector holding websocket dropped, returning");
                      writer.close().await.unwrap_or_else(|err| error!("{}", err));
                      if incoming_sender
                        .send(ButtplugTransportIncomingMessage::Close("Server closed connection".to_owned()))
                        .await
                        .is_err()
                      {
                        warn!("Websocket holder has closed, exiting websocket loop.");
                      }
                      return;
                    }
                  },
                  response = reader.next().fuse() => {
                    trace!("Websocket receiving: {:?}", response);
                    if response.is_none() {
                      info!("Connector holding websocket dropped, returning");
                      writer.close().await.unwrap_or_else(|err| error!("{}", err));
                      return;
                    }
                    match response.expect("Already checked for none.") {
                      Ok(msg) => match msg {
                        Message::Text(t) => {
                          if incoming_sender
                            .send(ButtplugTransportIncomingMessage::Message(
                              ButtplugSerializedMessage::Text(t.to_string()),
                            ))
                            .await
                            .is_err()
                          {
                            warn!("Websocket holder has closed, exiting websocket loop.");                                
                            return;
                          }
                        }
                        Message::Binary(v) => {
                          if incoming_sender
                            .send(ButtplugTransportIncomingMessage::Message(
                              ButtplugSerializedMessage::Binary(v),
                            ))
                            .await
                            .is_err()
                          {
                            warn!("Websocket holder has closed, exiting websocket loop.");
                            return;
                          }
                        }
                        Message::Ping(data) => {
                          writer.send(Message::Pong(data)).await.expect("This should never fail?");
                        }
                        Message::Pong(_) => {}
                        Message::Frame(_) => {}
                        Message::Close(_) => {
                          info!("Websocket has requested close.");
                          if incoming_sender
                            .send(ButtplugTransportIncomingMessage::Close("Server closed connection".to_owned()))
                            .await
                            .is_err()
                          {
                            warn!("Websocket holder has closed, exiting websocket loop.");
                            return;
                          }
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
                  _ = disconnect_notifier.notified().fuse() => {
                    // If we can't close, just print the error to the logs but
                    // still break out of the loop.
                    //
                    // TODO Emit a full error here that should bubble up to the client.
                    info!("Websocket requested to disconnect.");
                    writer.close().await.unwrap_or_else(|err| error!("{}", err));
                    if incoming_sender
                      .send(ButtplugTransportIncomingMessage::Close("Disconnect notifier triggered, closed connection".to_owned()))
                      .await
                      .is_err()
                    {
                      warn!("Websocket holder has closed, exiting websocket loop.");
                      return;
                    }
                    return;
                  }
                }
              }
            }
            .instrument(tracing::info_span!("Websocket Client I/O Task")),
          );
          Ok(())
        }
        Err(websocket_error) => Err(ButtplugConnectorError::TransportSpecificError(
          ButtplugConnectorTransportSpecificError::TungsteniteError(websocket_error),
        )),
      }
    }
    .boxed()
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    let disconnect_notifier = self.disconnect_notifier;
    async move {
      // If we can't send the message, we have no loop, so we're not connected.
      disconnect_notifier.notify_waiters();
      Ok(())
    }
    .boxed()
  }
}
