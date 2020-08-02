use crate::{
  connector::{
    transport::{
      ButtplugConnectorTransport, ButtplugConnectorTransportConnectResult, ButtplugTransportMessage, ButtplugConnectorTransportSpecificError
    },
    ButtplugConnectorResultFuture, ButtplugConnectorError
  },
  core::messages::serializer::ButtplugSerializedMessage,
  util::async_manager,
};
use async_channel::{bounded, Receiver, Sender};
#[cfg(feature = "async-std-runtime")]
use async_std::net::TcpListener;
use async_tls::TlsAcceptor;
use futures::{
  future::{self, select_all, BoxFuture},
  AsyncRead, AsyncWrite, FutureExt, SinkExt, StreamExt,
};
use rustls::{
  internal::pemfile::{certs, pkcs8_private_keys},
  NoClientAuth, ServerConfig,
};
use std::{
  fs::File,
  io::BufReader,
  sync::Arc,
};

#[derive(Default, Clone, Debug)]
pub struct ButtplugWebsocketServerTransportOptions {
  pub ws_listen_on_all_interfaces: bool,
  pub ws_insecure_port: Option<u16>,
  pub ws_secure_port: Option<u16>,
  pub ws_cert_file: Option<String>,
  pub ws_priv_file: Option<String>,
}

async fn accept_connection<S>(
  stream: S,
  mut request_receiver: Receiver<ButtplugSerializedMessage>,
  response_sender: Sender<ButtplugTransportMessage>,
) where
  S: AsyncRead + AsyncWrite + Unpin,
{
  let ws_stream = async_tungstenite::accept_async(stream)
    .await
    .expect("Error during the websocket handshake occurred");

  info!("New WebSocket connection.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  loop {
    select! {
      serialized_msg = request_receiver.next().fuse() => match serialized_msg {
        Some(msg) => match msg {
          ButtplugSerializedMessage::Text(text_msg) => websocket_server_sender
            .send(async_tungstenite::tungstenite::Message::Text(text_msg))
            .await
            .unwrap(),
          ButtplugSerializedMessage::Binary(binary_msg) => websocket_server_sender
            .send(async_tungstenite::tungstenite::Message::Binary(binary_msg))
            .await
            .unwrap()
        },
        None => {
          error!("Server disappeared, breaking.");
          return;
        }
      },
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        // TODO should match instead of unwrap here in case there's a socket error.
        Some(msg) => match msg.unwrap() {
          async_tungstenite::tungstenite::Message::Text(text_msg) => {
            info!("Got text: {}", text_msg);
            if response_sender.send(ButtplugTransportMessage::Message(ButtplugSerializedMessage::Text(text_msg))).await.is_err() {
              error!("Connector that owns transport no longer available, exiting.");
              break;
            }
          }
          async_tungstenite::tungstenite::Message::Close(_) => {
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
            panic!("Don't know how to handle binary message types!");
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
}

impl ButtplugWebsocketServerTransport {
  pub fn new(options: ButtplugWebsocketServerTransportOptions) -> Self {
    Self { options }
  }
}

impl ButtplugConnectorTransport for ButtplugWebsocketServerTransport {
  fn connect(&self) -> ButtplugConnectorTransportConnectResult {
    let (request_sender, request_receiver) = bounded(256);
    let (response_sender, response_receiver) = bounded(256);
    let mut tasks: Vec<BoxFuture<'static, Result<(), ButtplugConnectorError>>> = vec![];

    if let Some(ws_insecure_port) = self.options.ws_insecure_port {
      let addr = format!("127.0.0.1:{}", ws_insecure_port);
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
          async_manager::spawn(async move {
            accept_connection(stream, request_receiver_clone, response_sender_clone).await;
          }).unwrap();
          Ok(())
        } else {
          Err(ButtplugConnectorError::ConnectorGenericError("Could not run accept for insecure port".to_owned()))
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
          return Err(ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("No cert file provided".to_owned())));
        }

        let cert_file = File::open(options.ws_cert_file.unwrap())
          .map_err(|_| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("Specified cert file does not exist or cannot be opened".to_owned())))?;
        let certs = certs(&mut BufReader::new(cert_file))
          .map_err(|_| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("Specified cert file cannot load correctly".to_owned())))?;

        if options.ws_priv_file.is_none() {
          return Err(ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("No private key file provided".to_owned())));
        }
  
        let key_file = File::open(options.ws_priv_file.unwrap())
          .map_err(|_| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("Specified private key file does not exist or cannot be opened".to_owned())))?;
        let mut keys = pkcs8_private_keys(&mut BufReader::new(key_file))
          .map_err(|_| {
            ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("Specified private key file cannot load correctly".to_owned()))
          })?;

        // we don't use client authentication
        let mut config = ServerConfig::new(NoClientAuth::new());
        config
          // set this server to use one cert together with the loaded private key
          .set_single_cert(certs, keys.remove(0))
          .map_err(|_| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::SecureServerError("Cannot set up cert with provided cert/key pair due to TLS Error".to_owned())))?;
        let acceptor = TlsAcceptor::from(Arc::new(config));

        let addr = format!("127.0.0.1:{}", ws_secure_port);
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
          let tls_stream = handshake.await.unwrap();
          info!("Websocket Secure: Got connection");
          async_manager::spawn(async move {
            accept_connection(tls_stream, request_receiver_clone, response_sender_clone).await;
          }).unwrap();
          Ok(())
        } else {
          Err(ButtplugConnectorError::ConnectorGenericError("Could not run accept for insecure port".to_owned()))
        }
      };
      tasks.push(Box::pin(fut));
    }
    
    Box::pin(async move {
      if let Err(connector_err) = select_all(tasks).await.0 {
        Err(connector_err)
      } else {
        Ok((request_sender, response_receiver))
      }
    })
  }

  fn disconnect(self) -> ButtplugConnectorResultFuture {
    // TODO We should definitely allow people to disconnect. That would be a good thing.
    Box::pin(future::ready(Ok(())))
  }
}
