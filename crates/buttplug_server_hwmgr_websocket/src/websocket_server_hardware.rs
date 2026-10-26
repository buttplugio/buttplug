// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::websocket_server_comm_manager::WebsocketServerDeviceCommManagerInitInfo;
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server::device::hardware::{
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
};
use buttplug_server_device_config::{Endpoint, ProtocolCommunicationSpecifier, WebsocketSpecifier};
use futures::{
  FutureExt,
  SinkExt,
  StreamExt,
  future::{self, BoxFuture},
};
use std::{
  fmt::{self, Debug},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};
use tokio::{
  net::TcpStream,
  select,
  sync::{
    Mutex,
    broadcast,
    mpsc::{Receiver, Sender, channel},
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
      _ = sleep(Duration::from_millis(10000)) => {
        if pong_count == 0 {
          error!("No pongs received, considering connection closed.");
          break;
        }
        pong_count = 0;
        if websocket_server_sender
          .send(tokio_tungstenite::tungstenite::Message::Ping(vec!(0).into()))
          .await
          .is_err() {
          error!("Cannot send ping to client, considering connection closed.");
          break;
        }
      }
      ws_msg = request_receiver.recv() => {
        if let Some(binary_msg) = ws_msg {
          if websocket_server_sender
            .send(tokio_tungstenite::tungstenite::Message::Binary(binary_msg.into()))
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
      websocket_server_msg = websocket_server_receiver.next() => match websocket_server_msg {
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
                  let _ = response_sender.send(binary_msg.to_vec());
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
    buttplug_core::spawn!(async move {
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
    ProtocolCommunicationSpecifier::Websocket(WebsocketSpecifier::new(self.info.identifier()))
  }

  async fn connect(&mut self) -> Result<Box<dyn HardwareSpecializer>, ButtplugDeviceError> {
    let hardware_internal = WebsocketServerHardware::new(
      self.device_event_sender.clone(),
      self.info.clone(),
      self.outgoing_sender.clone(),
      self.incoming_broadcaster.clone(),
    );
    let mut endpoints = vec![Endpoint::Rx, Endpoint::Tx];
    for endpoint in hardware_internal.read_endpoints() {
      if !endpoints.contains(&endpoint) {
        endpoints.push(endpoint);
      }
    }
    let hardware = Hardware::new(
      self.info.identifier(),
      self.info.address(),
      &endpoints,
      &None,
      false,
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

  /// Endpoints with a canned read answer from the init packet, with their bytes. Entries whose
  /// endpoint name or hex string is invalid are skipped with a warning.
  fn canned_reads(&self) -> Vec<(Endpoint, Vec<u8>)> {
    self
      .info
      .reads()
      .iter()
      .filter_map(|(name, hex)| {
        let endpoint = match name.parse::<Endpoint>() {
          Ok(endpoint) => endpoint,
          Err(_) => {
            warn!("Ignoring canned read for unknown endpoint \"{name}\"");
            return None;
          }
        };
        match parse_hex(hex) {
          Some(bytes) => Some((endpoint, bytes)),
          None => {
            warn!("Ignoring canned read for endpoint \"{name}\": \"{hex}\" is not a hex string");
            None
          }
        }
      })
      .collect()
  }

  fn read_endpoints(&self) -> Vec<Endpoint> {
    self.canned_reads().into_iter().map(|(endpoint, _)| endpoint).collect()
  }
}

fn parse_hex(hex: &str) -> Option<Vec<u8>> {
  let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
  if hex.len() % 2 != 0 {
    return None;
  }
  (0..hex.len())
    .step_by(2)
    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
    .collect()
}

impl HardwareInternal for WebsocketServerHardware {
  fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device_event_sender.subscribe()
  }

  fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let connected = self.connected.clone();
    async move {
      connected.store(false, Ordering::Relaxed);
      Ok(())
    }
    .boxed()
  }

  fn read_value(
    &self,
    msg: &HardwareReadCmd,
  ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
    let endpoint = msg.endpoint();
    let canned = self
      .canned_reads()
      .into_iter()
      .find(|(e, _)| *e == endpoint)
      .map(|(_, bytes)| bytes);
    future::ready(match canned {
      Some(bytes) => Ok(HardwareReading::new(endpoint, &bytes)),
      None => Err(ButtplugDeviceError::UnhandledCommand(
        "Websocket Hardware does not support read".to_owned(),
      )),
    })
    .boxed()
  }

  fn write_value(
    &self,
    msg: &HardwareWriteCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    let sender = self.outgoing_sender.clone();
    let data = msg.data().clone();
    // TODO Should check endpoint validity
    async move {
      sender.send(data).await.map_err(|err| {
        ButtplugDeviceError::DeviceCommunicationError(format!(
          "Could not write value to websocket device: {err}"
        ))
      })
    }
    .boxed()
  }

  fn subscribe(
    &self,
    _msg: &HardwareSubscribeCmd,
  ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
    if self.subscribed.load(Ordering::Relaxed) {
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
      subscribed.store(true, Ordering::Relaxed);
      let token = CancellationToken::new();
      *(subscribed_token.lock().await) = Some(token.child_token());
      buttplug_core::spawn!("WebsocketServerHardware subscription", async move {
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
    if self.subscribed.load(Ordering::Relaxed) {
      let subscribed = self.subscribed.clone();
      let subscribed_token = self.subscribe_token.clone();
      async move {
        subscribed.store(false, Ordering::Relaxed);
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

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn init_info_without_reads_still_parses() {
    let info: WebsocketServerDeviceCommManagerInitInfo =
      serde_json::from_str(r#"{"identifier":"x","address":"y","version":1}"#).unwrap();
    assert!(info.reads().is_empty());
  }

  #[test]
  fn init_info_with_reads_parses() {
    let info: WebsocketServerDeviceCommManagerInitInfo = serde_json::from_str(
      r#"{"identifier":"HISMITH S2","address":"fake","version":1,"reads":{"rxblemodel":"1101"}}"#,
    )
    .unwrap();
    assert_eq!(info.reads().get("rxblemodel").unwrap(), "1101");
  }

  #[test]
  fn hex_parsing() {
    assert_eq!(parse_hex("1101"), Some(vec![0x11, 0x01]));
    assert_eq!(parse_hex("40 02"), Some(vec![0x40, 0x02]));
    assert_eq!(parse_hex("abc"), None);
    assert_eq!(parse_hex("zz"), None);
    assert_eq!(parse_hex(""), Some(vec![]));
  }

  fn hardware(reads: &str) -> WebsocketServerHardware {
    let info: WebsocketServerDeviceCommManagerInitInfo = serde_json::from_str(&format!(
      r#"{{"identifier":"HISMITH S2","address":"fake","version":1,"reads":{reads}}}"#
    ))
    .unwrap();
    let (event_sender, _) = broadcast::channel(4);
    let (outgoing_sender, _outgoing_receiver) = channel(4);
    let (incoming_broadcaster, _) = broadcast::channel(4);
    WebsocketServerHardware::new(event_sender, info, outgoing_sender, incoming_broadcaster)
  }

  #[test]
  fn canned_read_is_served_and_others_still_fail() {
    use futures::executor::block_on;
    let hardware = hardware(r#"{"rxblemodel":"1101","bogus":"01","rx":"xyz"}"#);
    assert_eq!(hardware.read_endpoints(), vec![Endpoint::RxBLEModel]);
    let reading = block_on(hardware.read_value(&HardwareReadCmd::new(
      uuid::Uuid::nil(),
      Endpoint::RxBLEModel,
      128,
      500,
    )))
    .unwrap();
    assert_eq!(reading.data(), &vec![0x11u8, 0x01]);
    assert!(
      block_on(hardware.read_value(&HardwareReadCmd::new(
        uuid::Uuid::nil(),
        Endpoint::Rx,
        128,
        500
      )))
      .is_err()
    );
  }
}
