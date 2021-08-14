use super::websocket_server_comm_manager::WebsocketServerDeviceCommManagerInitInfo;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, WebsocketSpecifier},
    ButtplugDeviceEvent, ButtplugDeviceImplCreator, DeviceImpl, DeviceImplInternal, DeviceReadCmd,
    DeviceSubscribeCmd, DeviceUnsubscribeCmd, DeviceWriteCmd, Endpoint,
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::{
  future::{self, BoxFuture},
  AsyncRead, AsyncWrite, FutureExt, SinkExt, StreamExt,
};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{
  broadcast,
  mpsc::{channel, Receiver, Sender},
  Mutex,
};
use tokio_util::sync::CancellationToken;

async fn run_connection_loop<S>(
  address: &str,
  event_sender: broadcast::Sender<ButtplugDeviceEvent>,
  ws_stream: async_tungstenite::WebSocketStream<S>,
  mut request_receiver: Receiver<Vec<u8>>,
  response_sender: broadcast::Sender<Vec<u8>>,
) where
  S: AsyncRead + AsyncWrite + Unpin,
{
  info!("Starting websocket server connection event loop.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  loop {
    select! {
      ws_msg = request_receiver.recv().fuse() => {
        if let Some(binary_msg) = ws_msg {
          if websocket_server_sender
            .send(async_tungstenite::tungstenite::Message::Binary(binary_msg))
            .await
            .is_err() {
            error!("Cannot send binary value to server, considering connection closed.");
            return;
          }
        } else {
          info!("Websocket server connector owner dropped, disconnecting websocket connection.");
          if websocket_server_sender.close().await.is_err() {
            error!("Cannot close, assuming connection already closed");
          }
          return;
        }
      }
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        Some(ws_data) => {
          match ws_data {
            Ok(msg) => {
              match msg {
                async_tungstenite::tungstenite::Message::Text(text_msg) => {
                  trace!("Got text: {}", text_msg);
                }
                async_tungstenite::tungstenite::Message::Binary(binary_msg) => {
                  // If no one is listening, ignore output.
                  let _ = response_sender.send(binary_msg);
                }
                async_tungstenite::tungstenite::Message::Close(_) => {
                  event_sender
                  .send(ButtplugDeviceEvent::Removed(
                    address.to_owned()
                  ))
                  .unwrap();
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
              }
            },
            Err(err) => {
              error!("Error from websocket server, assuming disconnection: {:?}", err);
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
  debug!("Exiting Websocket Server Device control loop.");
}

pub struct WebsocketServerDeviceImplCreator {
  info: WebsocketServerDeviceCommManagerInitInfo,
  outgoing_sender: Option<Sender<Vec<u8>>>,
  incoming_broadcaster: Option<broadcast::Sender<Vec<u8>>>,
  device_event_sender: Option<broadcast::Sender<ButtplugDeviceEvent>>,
}

impl WebsocketServerDeviceImplCreator {
  pub fn new<S>(
    info: WebsocketServerDeviceCommManagerInitInfo,
    ws_stream: async_tungstenite::WebSocketStream<S>,
  ) -> Self
  where
    S: 'static + AsyncRead + AsyncWrite + Unpin + Send,
  {
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let (incoming_broadcaster, _) = broadcast::channel(256);
    let incoming_broadcaster_clone = incoming_broadcaster.clone();
    let (device_event_sender, _) = broadcast::channel(256);
    let device_event_sender_clone = device_event_sender.clone();
    let address = info.address.clone();
    tokio::spawn(async move {
      run_connection_loop(
        &address,
        device_event_sender_clone,
        ws_stream,
        outgoing_receiver,
        incoming_broadcaster_clone,
      )
      .await;
    });
    Self {
      info,
      outgoing_sender: Some(outgoing_sender),
      incoming_broadcaster: Some(incoming_broadcaster),
      device_event_sender: Some(device_event_sender),
    }
  }
}

impl Debug for WebsocketServerDeviceImplCreator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("WebsocketServerDeviceImplCreator")
      .field("info", &self.info)
      .finish()
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for WebsocketServerDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    DeviceSpecifier::Websocket(WebsocketSpecifier::new(&self.info.identifier))
  }

  async fn try_create_device_impl(
    &mut self,
    _: ProtocolDefinition,
  ) -> Result<DeviceImpl, ButtplugError> {
    let device_impl_internal = WebsocketServerDeviceImpl::new(
      self.device_event_sender.take().unwrap(),
      self.info.clone(),
      self.outgoing_sender.take().unwrap(),
      self.incoming_broadcaster.take().unwrap(),
    );
    let device_impl = DeviceImpl::new(
      &self.info.identifier,
      &self.info.address,
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(device_impl_internal),
    );
    Ok(device_impl)
  }
}

pub struct WebsocketServerDeviceImpl {
  connected: Arc<AtomicBool>,
  subscribed: Arc<AtomicBool>,
  subscribe_token: Arc<Mutex<Option<CancellationToken>>>,
  info: WebsocketServerDeviceCommManagerInitInfo,
  outgoing_sender: Sender<Vec<u8>>,
  incoming_broadcaster: broadcast::Sender<Vec<u8>>,
  device_event_sender: broadcast::Sender<ButtplugDeviceEvent>,
}

impl WebsocketServerDeviceImpl {
  pub fn new(
    device_event_sender: broadcast::Sender<ButtplugDeviceEvent>,
    info: WebsocketServerDeviceCommManagerInitInfo,
    outgoing_sender: Sender<Vec<u8>>,
    incoming_broadcaster: broadcast::Sender<Vec<u8>>,
  ) -> Self {
    Self {
      connected: Arc::new(AtomicBool::new(true)),
      info,
      outgoing_sender,
      incoming_broadcaster,
      device_event_sender,
      subscribed: Arc::new(AtomicBool::new(false)),
      subscribe_token: Arc::new(Mutex::new(None)),
    }
  }
}

impl DeviceImplInternal for WebsocketServerDeviceImpl {
  fn event_stream(&self) -> broadcast::Receiver<ButtplugDeviceEvent> {
    self.device_event_sender.subscribe()
  }

  fn connected(&self) -> bool {
    self.connected.load(Ordering::SeqCst)
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    let connected = self.connected.clone();
    Box::pin(async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    })
  }

  fn read_value(
    &self,
    _msg: DeviceReadCmd,
  ) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    unimplemented!("Not implemented for websockets");
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let sender = self.outgoing_sender.clone();
    // TODO Should check endpoint validity
    Box::pin(async move {
      sender.send(msg.data).await.unwrap();
      Ok(())
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    if self.subscribed.load(Ordering::SeqCst) {
      return Box::pin(future::ready(Ok(())));
    }
    // TODO Should check endpoint validity
    let mut data_receiver = self.incoming_broadcaster.subscribe();
    let event_sender = self.device_event_sender.clone();
    let address = self.info.address.clone();
    let subscribed = self.subscribed.clone();
    let subscribed_token = self.subscribe_token.clone();
    Box::pin(async move {
      subscribed.store(true, Ordering::SeqCst);
      let token = CancellationToken::new();
      *(subscribed_token.lock().await) = Some(token.child_token());
      async_manager::spawn(async move {
        loop {
          select! {
            result = data_receiver.recv().fuse() => {
              match result {
                Ok(data) => {
                  info!("Got websocket data! {:?}", data);
                  event_sender
                    .send(ButtplugDeviceEvent::Notification(
                      address.clone(),
                      Endpoint::Tx,
                      data,
                    ))
                    .unwrap();
                },
                Err(_) => break,
              }
            },
            _ = token.cancelled().fuse() => {
              break;
            }
          }
        }
        info!("Data channel closed, ending websocket server device listener task");
      })
      .unwrap();
      Ok(())
    })
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    if self.subscribed.load(Ordering::SeqCst) {
      let subscribed = self.subscribed.clone();
      let subscribed_token = self.subscribe_token.clone();
      Box::pin(async move {
        subscribed.store(false, Ordering::SeqCst);
        let token = (subscribed_token.lock().await).take().unwrap();
        token.cancel();
        Ok(())
      })
    } else {
      Box::pin(future::ready(Err(
        ButtplugDeviceError::DeviceCommunicationError("Device not subscribed.".to_owned()).into(),
      )))
    }
  }
}
