// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::websocket_server_hardware::WebsocketServerHardwareConnector;
use crate::{
  core::ButtplugResultFuture,
  server::device::hardware::communication::{
    HardwareCommunicationManager,
    HardwareCommunicationManagerBuilder,
    HardwareCommunicationManagerEvent,
  },
  util::async_manager,
};
use futures::{FutureExt, StreamExt};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use tokio::{net::TcpListener, sync::mpsc::Sender};
use tokio_util::sync::CancellationToken;

// Packet format received from external devices.
#[derive(Serialize, Deserialize, Debug, Clone, Getters, CopyGetters)]
pub struct WebsocketServerDeviceCommManagerInitInfo {
  #[getset(get = "pub")]
  identifier: String,
  #[getset(get = "pub")]
  address: String,
  #[getset(get_copy = "pub")]
  version: u32,
}

#[derive(Clone)]
pub struct WebsocketServerDeviceCommunicationManagerBuilder {
  listen_on_all_interfaces: bool,
  server_port: u16,
}

impl Default for WebsocketServerDeviceCommunicationManagerBuilder {
  fn default() -> Self {
    Self {
      listen_on_all_interfaces: false,
      server_port: 54817,
    }
  }
}

impl WebsocketServerDeviceCommunicationManagerBuilder {
  pub fn listen_on_all_interfaces(mut self, should_listen: bool) -> Self {
    self.listen_on_all_interfaces = should_listen;
    self
  }

  pub fn server_port(mut self, port: u16) -> Self {
    self.server_port = port;
    self
  }
}

impl HardwareCommunicationManagerBuilder for WebsocketServerDeviceCommunicationManagerBuilder {
  fn finish(
    &mut self,
    sender: Sender<HardwareCommunicationManagerEvent>,
  ) -> Box<dyn HardwareCommunicationManager> {
    Box::new(WebsocketServerDeviceCommunicationManager::new(
      sender,
      self.server_port,
      self.listen_on_all_interfaces,
    ))
  }
}

pub struct WebsocketServerDeviceCommunicationManager {
  server_cancellation_token: CancellationToken,
}

impl WebsocketServerDeviceCommunicationManager {
  fn new(
    sender: Sender<HardwareCommunicationManagerEvent>,
    port: u16,
    listen_on_all_interfaces: bool,
  ) -> Self {
    trace!("Websocket server port created.");
    let server_cancellation_token = CancellationToken::new();
    let child_token = server_cancellation_token.child_token();
    async_manager::spawn(async move {
      let base_addr = if listen_on_all_interfaces {
        "0.0.0.0"
      } else {
        "127.0.0.1"
      };

      let addr = format!("{}:{}", base_addr, port);
      debug!("Trying to listen on {}", addr);

      // Create the event loop and TCP listener we'll accept connections on.
      debug!("Socket bound.");
      let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(err) => {
          error!("Cannot bind websocket server to {}: {:?}.", addr, err);
          return;
        }
      };
      debug!("Listening on: {}", addr);
      loop {
        select! {
          listener_result = listener.accept().fuse() => {
            let stream = if let Ok((stream, _)) = listener_result {
              stream
            } else {
              error!("Cannot bind websocket server comm manager to address {}.", addr);
              return;
            };
            info!("Got connection");
            let ws_fut = tokio_tungstenite::accept_async(stream);
            let mut ws_stream = match ws_fut.await {
              Ok(ws_stream) => ws_stream,
              Err(err) => {
                error!("Cannot accept socket: {}", err);
                continue;
              }
            };
            // Websockets are different from the rest of the communication managers, in that we have no
            // information about the device type when we create the connection, and therefore have to
            // wait for the first packet. We'll have to pass our device event sender off to the newly
            // created event loop, so that it can fire once the info packet is received.
            let sender_clone = sender.clone();
            tokio::spawn(async move {
              // TODO Implement a receive timeout here so we don't wait forever
              if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(info_message))) =
                ws_stream.next().await
              {
                let info_packet: WebsocketServerDeviceCommManagerInitInfo =
                  if let Ok(packet) = serde_json::from_str(&info_message) {
                    packet
                  } else {
                    error!("Did not receive a valid JSON info packet as the first packet, disconnecting.");
                    if let Err(err) = ws_stream.close(None).await {
                      error!("Error closing connection: {}", err);
                    }
                    return;
                  };
                if sender_clone
                  .send(HardwareCommunicationManagerEvent::DeviceFound {
                    name: format!("Websocket Device {}", info_packet.identifier),
                    address: info_packet.address.clone(),
                    creator: Box::new(WebsocketServerHardwareConnector::new(
                      info_packet,
                      ws_stream,
                    )),
                  })
                  .await
                  .is_err()
                {
                  error!("Device manager disappeared, exiting.");
                }
              } else {
                error!("Did not receive info message as first packet, dropping connection.");
              }
            });
          },
          _ = child_token.cancelled().fuse() => {
            info!("Task token cancelled, assuming websocket server comm manager shutdown.");
            break;
          }
        }
      }
    });
    Self {
      server_cancellation_token,
    }
  }
}

impl HardwareCommunicationManager for WebsocketServerDeviceCommunicationManager {
  fn name(&self) -> &'static str {
    "WebsocketServerCommunicationManager"
  }

  fn start_scanning(&mut self) -> ButtplugResultFuture {
    debug!("Websocket server manager scanning for devices.");
    async move { Ok(()) }.boxed()
  }

  fn stop_scanning(&mut self) -> ButtplugResultFuture {
    async move { Ok(()) }.boxed()
  }

  // No restrictions since this is network not hardware.
  fn can_scan(&self) -> bool {
    true
  }
}

impl Drop for WebsocketServerDeviceCommunicationManager {
  fn drop(&mut self) {
    self.server_cancellation_token.cancel();
  }
}
