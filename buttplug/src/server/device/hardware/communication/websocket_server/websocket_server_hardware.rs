// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::websocket_server_comm_manager::WebsocketServerDeviceCommManagerInitInfo;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, WebsocketSpecifier},
    hardware::{
      GenericHardwareSpecializer,
      Hardware,
      HardwareConnector,
      HardwareEvent,
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSpecializer,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use futures::{
  future::{self, BoxFuture},
  FutureExt,
  SinkExt,
  StreamExt,
};
use std::{
  fmt::{self, Debug},
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::{
  net::TcpStream,
  sync::{
    broadcast,
    mpsc::{channel, Receiver, Sender},
    Mutex,
  },
  time::sleep,
};
use tokio_util::sync::CancellationToken;

async fn run_connection_loop(
  address: &str,
  event_sender: broadcast::Sender<HardwareEvent>,
  ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
  mut request_receiver: Receiver<Vec<u8>>,
  response_sender: broadcast::Sender<Vec<u8>>,
) {
  info!("Starting websocket server connection event loop.");

  let (mut websocket_server_sender, mut websocket_server_receiver) = ws_stream.split();

  // Start pong count at 1, so we'll clear it after sending our first ping.
  let mut pong_count = 1u32;

  loop {
    select! {
      _ = sleep(Duration::from_millis(10000)).fuse() => {
        if pong_count == 0 {
          error!("No pongs received, considering connection closed.");
          break;
        }
        pong_count = 0;
        if websocket_server_sender
          .send(tokio_tungstenite::tungstenite::Message::Ping(vec!(0)))
          .await
          .is_err() {
          error!("Cannot send ping to client, considering connection closed.");
          break;
        }
      }
      ws_msg = request_receiver.recv().fuse() => {
        if let Some(binary_msg) = ws_msg {
          if websocket_server_sender
            .send(tokio_tungstenite::tungstenite::Message::Binary(binary_msg))
            .await
            .is_err() {
            error!("Cannot send binary value to client, considering connection closed.");
            break;
          }
        } else {
          info!("Websocket server connector owner dropped, disconnecting websocket connection.");
          break;
        }
      }
      websocket_server_msg = websocket_server_receiver.next().fuse() => match websocket_server_msg {
        Some(ws_data) => {
          match ws_data {
            Ok(msg) => {
              match msg {
                tokio_tungstenite::tungstenite::Message::Text(text_msg) => {
                  // If someone accidentally packs text, politely turn it into binary for them.
                  let _ = response_sender.send(text_msg.as_bytes().to_vec());
                }
                tokio_tungstenite::tungstenite::Message::Binary(binary_msg) => {
                  // If no one is listening, ignore output.
                  let _ = response_sender.send(binary_msg);
                }
                tokio_tungstenite::tungstenite::Message::Close(_) => {
                  // Drop the error if no one receives the message, we're breaking anyways.
                  let _ = event_sender
                    .send(HardwareEvent::Disconnected(
                      address.to_owned()
                    ));
                  break;
                }
                tokio_tungstenite::tungstenite::Message::Ping(_) => {
                  // noop
                  continue;
                }
                tokio_tungstenite::tungstenite::Message::Frame(_) => {
                  // noop
                  continue;
                }
                tokio_tungstenite::tungstenite::Message::Pong(_) => {
                  pong_count += 1;
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
          break;
        }
      }
    }
  }

  if let Err(e) = websocket_server_sender.close().await {
    error!("Error closing websocket: {}", e);
  }
  debug!("Exiting Websocket Server Device control loop.");
}

impl Debug for WebsocketServerHardwareConnector {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("WebsocketServerHardwareConnector")
      .field("info", &self.info)
      .finish()
  }
}

pub struct WebsocketServerHardwareConnector {
  info: WebsocketServerDeviceCommManagerInitInfo,
  outgoing_sender: Sender<Vec<u8>>,
  incoming_broadcaster: broadcast::Sender<Vec<u8>>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
}

impl WebsocketServerHardwareConnector {
  pub fn new(
    info: WebsocketServerDeviceCommManagerInitInfo,
    ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
  ) -> Self {
    let (outgoing_sender, outgoing_receiver) = channel(256);
    let (incoming_broadcaster, _) = broadcast::channel(256);
    let incoming_broadcaster_clone = incoming_broadcaster.clone();
    let (device_event_sender, _) = broadcast::channel(256);
    let device_event_sender_clone = device_event_sender.clone();
    let address = info.address().clone();
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
      outgoing_sender,
      incoming_broadcaster,
      device_event_sender,
    }
  }
}

#[async_trait]
impl HardwareConnector for WebsocketServerHardwareConnector {
  fn specifier(&self) -> ProtocolCommunicationSpecifier {
    ProtocolCommunicationSpecifier::Websocket(WebsocketSpecifier::new(&vec![self
      .info
      .identifier()
      .to_owned()]))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let hardware_internal = WebsocketServerHardware::new(
      self.device_event_sender.clone(),
      self.info.clone(),
      self.outgoing_sender.clone(),
      self.incoming_broadcaster.clone(),
    );
    let hardware = Hardware::new(
      self.info.identifier(),
      self.info.address(),
      &[Endpoint::Rx, Endpoint::Tx],
      Box::new(hardware_internal),
    );
    Ok(Box::new(GenericHardwareSpecializer::new(hardware)))
  }
}

pub struct WebsocketServerHardware {
  connected: Arc<AtomicBool>,
  subscribed: Arc<AtomicBool>,
  subscribe_token: Arc<Mutex<Option<CancellationToken>>>,
  info: WebsocketServerDeviceCommManagerInitInfo,
  outgoing_sender: Sender<Vec<u8>>,
  incoming_broadcaster: broadcast::Sender<Vec<u8>>,
  device_event_sender: broadcast::Sender<HardwareEvent>,
}

impl WebsocketServerHardware {
  pub fn new(
    device_event_sender: broadcast::Sender<HardwareEvent>,
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

impl HardwareInternal for WebsocketServerHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device_event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    async move {
      connected.store(false, Ordering::SeqCst);
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    _msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Websocket Hardware does not support read".to_owned(),
    )))
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.outgoing_sender.clone();
    let data = msg.data.clone();
    // TODO Should check endpoint validity
    async move {
      sender.send(data).await.map_err(|err| {
        ButtplugDeviceError::DeviceCommunicationError(format!(
          "Could not write value to websocket device: {}",
          err
        ))
      })
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if self.subscribed.load(Ordering::SeqCst) {
      error!("Endpoint already subscribed somehow!");
      return future::ready(Ok(())).boxed();
    }
    // TODO Should check endpoint validity
    let mut data_receiver = self.incoming_broadcaster.subscribe();
    let event_sender = self.device_event_sender.clone();
    let address = self.info.address().clone();
    let subscribed = self.subscribed.clone();
    let subscribed_token = self.subscribe_token.clone();
    async move {
      subscribed.store(true, Ordering::SeqCst);
      let token = CancellationToken::new();
      *(subscribed_token.lock().await) = Some(token.child_token());
      async_manager::spawn(async move {
        loop {
          select! {
            result = data_receiver.recv().fuse() => {
              match result {
                Ok(data) => {
                  debug!("Got websocket data! {:?}", data);
                  // We don't really care if there's no one to send the error to here.
                  let _ = event_sender
                    .send(HardwareEvent::Notification(
                      address.clone(),
                      Endpoint::Tx,
                      data,
                    ));
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
      });
      Ok(())
    }
    .boxed()
  }

  fn unsubscribe(
    &self,
    _msg: &HardwareUnsubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if self.subscribed.load(Ordering::SeqCst) {
      let subscribed = self.subscribed.clone();
      let subscribed_token = self.subscribe_token.clone();
      async move {
        subscribed.store(false, Ordering::SeqCst);
        let token = (subscribed_token.lock().await)
          .take()
          .expect("If we were subscribed, we'll have a token.");
        token.cancel();
        Ok(())
      }
      .boxed()
    } else {
      future::ready(Err(ButtplugDeviceError::DeviceCommunicationError(
        "Device not subscribed.".to_owned(),
      )))
      .boxed()
    }
  }
}
