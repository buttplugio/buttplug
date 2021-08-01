
use crate::{
  core::{
    errors::ButtplugError,
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
use futures::{future::BoxFuture, FutureExt, AsyncRead, AsyncWrite, StreamExt, SinkExt};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
};
use tokio::sync::{
  mpsc::{Receiver, Sender, channel},
  broadcast,
};
use super::websocket_server_comm_manager::WebsocketCommManagerInitInfo;


async fn run_connection_loop<S>(
  ws_stream: async_tungstenite::WebSocketStream<S>,
  mut request_receiver: Receiver<Vec<u8>>,
  response_sender: Sender<Vec<u8>>,
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
                async_tungstenite::tungstenite::Message::Binary(_) => {
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
}

pub struct WebsocketServerDeviceImplCreator {
  info: WebsocketCommManagerInitInfo,
  outgoing_sender: Option<Sender<Vec<u8>>>,
  incoming_receiver: Option<Receiver<Vec<u8>>>,
}

impl WebsocketServerDeviceImplCreator {
  pub fn new<S>(info: WebsocketCommManagerInitInfo, ws_stream: async_tungstenite::WebSocketStream<S>) -> Self where S: 'static + AsyncRead + AsyncWrite + Unpin + Send {
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let (incoming_sender, incoming_receiver) = channel(256);
    tokio::spawn(async move {
      run_connection_loop(ws_stream, outgoing_receiver, incoming_sender).await;
    });
    Self {
      info,
      outgoing_sender: Some(outgoing_sender),
      incoming_receiver: Some(incoming_receiver),
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
    protocol: ProtocolDefinition,
  ) -> Result<DeviceImpl, ButtplugError> {
    let device_impl_internal = WebsocketServerDeviceImpl::new(self.info.clone(), self.outgoing_sender.take().unwrap(), self.incoming_receiver.take().unwrap());
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
  info: WebsocketCommManagerInitInfo,
  outgoing_sender: Sender<Vec<u8>>,
  incoming_receiver: Receiver<Vec<u8>>,
  device_event_sender: broadcast::Sender<ButtplugDeviceEvent>,
}

impl WebsocketServerDeviceImpl {
  pub fn new(
    info: WebsocketCommManagerInitInfo,
    outgoing_sender: Sender<Vec<u8>>,
    incoming_receiver: Receiver<Vec<u8>>
  ) -> Self {
    let (device_event_sender, _) = broadcast::channel(256);
    Self {
      connected: Arc::new(AtomicBool::new(true)),
      info,
      outgoing_sender,
      incoming_receiver,
      device_event_sender
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
    unimplemented!("Not implemented for websockets");
    /*
    // TODO Should check endpoint validity
    let data_receiver = self.port_receiver.clone();
    let event_sender = self.device_event_sender.clone();
    let address = self.address.clone();
    Box::pin(async move {
      async_manager::spawn(async move {
        // TODO There's only one subscribable endpoint on a serial port, so we
        // should check to make sure we don't have multiple subscriptions so we
        // don't deadlock.
        let mut data_receiver_mut = data_receiver.lock().await;
        loop {
          match data_receiver_mut.recv().await {
            Some(data) => {
              info!("Got serial data! {:?}", data);
              event_sender
                .send(ButtplugDeviceEvent::Notification(
                  address.clone(),
                  Endpoint::Tx,
                  data,
                ))
                .unwrap();
            }
            None => {
              info!("Data channel closed, ending serial listener task");
              break;
            }
          }
        }
      })
      .unwrap();
      Ok(())
    })
    */
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    unimplemented!("Not implemented for websockets");
  }
}
