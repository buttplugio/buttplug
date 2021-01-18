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
#[cfg(feature = "async-std-runtime")]
use async_std::net::TcpListener;
use async_tls::TlsAcceptor;
use futures::{
  future::{select_all, BoxFuture},
  AsyncRead,
  AsyncWrite,
  FutureExt,
  SinkExt,
  StreamExt,
};
use rustls::{
  internal::pemfile::{certs, pkcs8_private_keys, rsa_private_keys},
  NoClientAuth,
  ServerConfig,
};
use std::{fs::File, io::BufReader, sync::Arc};
use tokio::sync::{
  mpsc::{channel, Receiver, Sender},
  Mutex,
};

#[derive(Default, Clone, Debug)]
pub struct ButtplugWebsocketServerTransportOptions {
  /// If true, listens all on available interfaces. Otherwise, only listens on 127.0.0.1.
  pub ws_listen_on_all_interfaces: bool,
  /// Insecure port for listening for websocket connections.
  pub ws_insecure_port: Option<u16>,
  /// Secure port for listen for websocket connections. Requires cert and key
  /// file options to be passed in also. For secure connections to localhost
  /// (i.e. from browsers that require secure localhost context to native
  /// buttplug-rs), certs should work for 127.0.0.1. Certs signed to "localhost"
  /// may work, but many Buttplug apps default to 127.0.0.1.
  pub ws_secure_port: Option<u16>,
  /// Certificate file for secure connections.
  pub ws_cert_file: Option<String>,
  /// Private key file for secure connections. Key must be > 1024 bit, and in
  /// either RSA or PKCS8 format.
  pub ws_priv_file: Option<String>,
}

async fn run_connection_loop<S>(
  ws_stream: async_tungstenite::WebSocketStream<S>,
  mut request_receiver: Receiver<ButtplugTransportOutgoingMessage>,
  response_sender: Sender<ButtplugTransportIncomingMessage>,
) where
  S: AsyncRead + AsyncWrite + Unpin,
{
  info!("Starting websocket server connection event loop.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  loop {
    select! {
      serialized_msg = request_receiver.recv().fuse() => match serialized_msg {
        Some(msg) => match msg {
          ButtplugTransportOutgoingMessage::Message(outgoing_msg) => {
            match outgoing_msg {
              ButtplugSerializedMessage::Text(text_msg) => {
                if websocket_server_sender
                    .send(async_tungstenite::tungstenite::Message::Text(text_msg))
                    .await
                    .is_err() {
                    error!("Cannot send text value to server, considering connection closed.");
                    return;
                  }
                }
              ButtplugSerializedMessage::Binary(binary_msg) => {
                if websocket_server_sender
                    .send(async_tungstenite::tungstenite::Message::Binary(binary_msg))
                    .await
                    .is_err() {
                    error!("Cannot send binary value to server, considering connection closed.");
                    return;
                  }
                }
              }
            },
            ButtplugTransportOutgoingMessage::Close => {
              if websocket_server_sender.close().await.is_err() {
                error!("Cannot close, assuming connection already closed");
                return;
              }
            }
        },
        None => {
          error!("Server disappeared, breaking.");
          return;
        }
      },
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        Some(ws_data) => {
          match ws_data {
            Ok(msg) => {
              match msg {
                async_tungstenite::tungstenite::Message::Text(text_msg) => {
                  debug!("Got text: {}", text_msg);
                  if response_sender.send(ButtplugTransportIncomingMessage::Message(ButtplugSerializedMessage::Text(text_msg))).await.is_err() {
                    error!("Connector that owns transport no longer available, exiting.");
                    break;
                  }
                }
                async_tungstenite::tungstenite::Message::Close(_) => {
                  let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
                  break;
                }
                async_tungstenite::tungstenite::Message::Ping(_) => {
                  // noop
                  continue;
                }
                async_tungstenite::tungstenite::Message::Pong(_) => {
                  // noop
                  continue;
                }
                async_tungstenite::tungstenite::Message::Binary(_) => {
                  error!("Don't know how to handle binary message types!");
                }
              }
            },
            Err(err) => {
              error!("Error from websocket server, assuming disconnection: {:?}", err);
              let _ = response_sender.send(ButtplugTransportIncomingMessage::Close("Websocket server closed".to_owned())).await;
              break;
            }
          }
        },
        None => {
          error!("Websocket channel closed, breaking");
          return;
        }
      }
    }
  }
}

/// Websocket connector for ButtplugClients, using [async_tungstenite]
pub struct ButtplugWebsocketServerTransport {
  options: ButtplugWebsocketServerTransportOptions,
  disconnect_sender: Arc<Mutex<Sender<ButtplugTransportOutgoingMessage>>>,
}

impl ButtplugWebsocketServerTransport {
  pub fn new(options: ButtplugWebsocketServerTransportOptions) -> Self {
    let (unused_sender, _) = channel(256);
    Self {
      options,
      disconnect_sender: Arc::new(Mutex::new(unused_sender)),
    }
  }
}

impl ButtplugConnectorTransport for ButtplugWebsocketServerTransport {
  fn connect(&self) -> ButtplugConnectorTransportConnectResult {
    let (request_sender, request_receiver_bare) = channel(256);
    let request_receiver = Arc::new(Mutex::new(Some(request_receiver_bare)));
    let (response_sender, response_receiver) = channel(256);
    let disconnect_sender = self.disconnect_sender.clone();
    let mut tasks: Vec<BoxFuture<'static, Result<(), ButtplugConnectorError>>> = vec![];

    let base_addr = if self.options.ws_listen_on_all_interfaces {
      "0.0.0.0"
    } else {
      "127.0.0.1"
    };

    if let Some(ws_insecure_port) = self.options.ws_insecure_port {
      let addr = format!("{}:{}", base_addr, ws_insecure_port);

      debug!("Websocket Insecure: Trying to listen on {}", addr);
      let request_receiver_clone = request_receiver.clone();
      let response_sender_clone = response_sender.clone();

      let fut = async move {
        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&addr).await;
        debug!("Websocket Insecure: Socket bound.");
        let listener = try_socket.expect("Failed to bind");
        debug!("Websocket Insecure: Listening on: {}", addr);

        if let Ok((stream, _)) = listener.accept().await {
          info!("Websocket Insecure: Got connection");
          let ws_stream = async_tungstenite::accept_async(stream)
            .await
            .map_err(|err| {
              error!("Websocket server accept error: {:?}", err);
              ButtplugConnectorError::TransportSpecificError(
                ButtplugConnectorTransportSpecificError::SecureServerError(format!(
                  "Error occurred during the websocket handshake: {:?}",
                  err
                )),
              )
            })?;

          async_manager::spawn(async move {
            run_connection_loop(
              ws_stream,
              (*request_receiver_clone.lock().await).take().unwrap(),
              response_sender_clone,
            )
            .await;
          })
          .unwrap();
          Ok(())
        } else {
          Err(ButtplugConnectorError::ConnectorGenericError(
            "Could not run accept for insecure port".to_owned(),
          ))
        }
      };
      tasks.push(Box::pin(fut));
    }

    if let Some(ws_secure_port) = self.options.ws_secure_port {
      let options = self.options.clone();
      let request_receiver_clone = request_receiver;
      let response_sender_clone = response_sender;

      let fut = async move {
        if options.ws_cert_file.is_none() {
          return Err(ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "No cert file provided".to_owned(),
            ),
          ));
        }

        info!("Loading cert file {:?}", options.ws_cert_file);
        let cert_file = File::open(options.ws_cert_file.unwrap()).map_err(|_| {
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "Specified cert file does not exist or cannot be opened".to_owned(),
            ),
          )
        })?;
        let certs = certs(&mut BufReader::new(cert_file)).map_err(|_| {
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "Specified cert file cannot load correctly".to_owned(),
            ),
          )
        })?;
        info!("Loaded certificate file");

        if options.ws_priv_file.is_none() {
          return Err(ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "No private key file provided".to_owned(),
            ),
          ));
        }

        info!("Loading RSA private key file {:?}", options.ws_priv_file);
        let rsa_key_file = File::open(options.ws_priv_file.clone().unwrap()).map_err(|_| {
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "Specified private key file does not exist or cannot be opened".to_owned(),
            ),
          )
        })?;

        let mut rsa_key_buf = BufReader::new(rsa_key_file);
        let mut keys = rsa_private_keys(&mut rsa_key_buf).map_err(|e| {
          error!("Cannot load RSA keys: {:?}", e);
          ButtplugConnectorError::TransportSpecificError(
            ButtplugConnectorTransportSpecificError::SecureServerError(
              "Specified private key file cannot load correctly".to_owned(),
            ),
          )
        })?;

        if keys.is_empty() {
          let pkcs8_key_file = File::open(options.ws_priv_file.unwrap()).map_err(|_| {
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::SecureServerError(
                "Specified private key file does not exist or cannot be opened".to_owned(),
              ),
            )
          })?;

          let mut pkcs8_key_buf = BufReader::new(pkcs8_key_file);
          keys = pkcs8_private_keys(&mut pkcs8_key_buf).map_err(|e| {
            error!("Cannot load PKCS8 keys: {:?}", e);
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::SecureServerError(
                "Specified private key file cannot load correctly".to_owned(),
              ),
            )
          })?;
          if keys.is_empty() {
            error!("No keys were loaded, cannot start secure server.");
            return Err(ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::SecureServerError(
                "Could not load private keys from file".to_owned(),
              ),
            ));
          }
        }
        info!("Loaded private key file");

        // we don't use client authentication
        let mut config = ServerConfig::new(NoClientAuth::new());
        config
          // set this server to use one cert together with the loaded private key
          .set_single_cert(certs, keys.remove(0))
          .map_err(|e| {
            error!("Secure cert config cannot set up: {:?}", e);
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::SecureServerError(
                "Cannot set up cert with provided cert/key pair due to TLS Error".to_owned(),
              ),
            )
          })?;
        let acceptor = TlsAcceptor::from(Arc::new(config));
        let addr = format!("{}:{}", base_addr, ws_secure_port);

        debug!("Websocket Secure: Trying to listen on {}", addr);
        // Create the event loop and TCP listener we'll accept connections on.
        let try_socket = TcpListener::bind(&addr).await;
        debug!("Websocket Secure: Socket bound.");
        let listener = try_socket.expect("Failed to bind");
        debug!("Websocket Secure: Listening on: {}", addr);

        if let Ok((stream, _)) = listener.accept().await {
          let handshake = acceptor.accept(stream);
          // The handshake is a future we can await to get an encrypted
          // stream back.
          let tls_stream = handshake.await.map_err(|e| {
            error!("Secure cert config cannot run handshake: {:?}", e);
            ButtplugConnectorError::TransportSpecificError(
              ButtplugConnectorTransportSpecificError::SecureServerError(format!("{:?}", e)),
            )
          })?;
          info!("Websocket Secure: Got connection");
          let ws_stream = async_tungstenite::accept_async(tls_stream)
            .await
            .map_err(|err| {
              error!("Websocket server accept error: {:?}", err);
              ButtplugConnectorError::TransportSpecificError(
                ButtplugConnectorTransportSpecificError::SecureServerError(format!(
                  "Error occurred during the websocket handshake: {:?}",
                  err
                )),
              )
            })?;
          async_manager::spawn(async move {
            run_connection_loop(
              ws_stream,
              (*request_receiver_clone.lock().await).take().unwrap(),
              response_sender_clone,
            )
            .await;
          })
          .unwrap();
          Ok(())
        } else {
          Err(ButtplugConnectorError::ConnectorGenericError(
            "Could not run accept for insecure port".to_owned(),
          ))
        }
      };
      tasks.push(Box::pin(fut));
    }

    Box::pin(async move {
      *disconnect_sender.lock().await = request_sender.clone();
      if tasks.len() == 0 {
        Err(ButtplugConnectorError::ConnectorGenericError("No ports specified for listening in websocket server connector.".to_owned()))
      } else if let Err(connector_err) = select_all(tasks).await.0 {
        Err(connector_err)
      } else {
        Ok((request_sender, response_receiver))
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
