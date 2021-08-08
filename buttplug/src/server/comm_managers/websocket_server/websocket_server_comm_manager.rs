use super::websocket_server_device_impl::WebsocketServerDeviceImplCreator;
use crate::{
  core::ButtplugResultFuture,
  server::comm_managers::{
    DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerBuilder,
  },
};
use futures::StreamExt;
use serde::Deserialize;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;

#[derive(Deserialize, Debug, Clone)]
pub struct WebsocketCommManagerInitInfo {
  pub identifier: String,
  pub address: String,
  pub version: u32,
}

pub struct WebsocketServerCommunicationManagerBuilder {
  sender: Option<tokio::sync::mpsc::Sender<DeviceCommunicationEvent>>,
  listen_on_all_interfaces: bool,
  server_port: u16,
}

impl Default for WebsocketServerCommunicationManagerBuilder {
  fn default() -> Self {
    Self {
      sender: None,
      listen_on_all_interfaces: false,
      server_port: 54817,
    }
  }
}

impl WebsocketServerCommunicationManagerBuilder {
  pub fn listen_on_all_interfaces(mut self, should_listen: bool) -> Self {
    self.listen_on_all_interfaces = should_listen;
    self
  }

  pub fn server_port(mut self, port: u16) -> Self {
    self.server_port = port;
    self
  }
}

impl DeviceCommunicationManagerBuilder for WebsocketServerCommunicationManagerBuilder {
  fn event_sender(mut self, sender: Sender<DeviceCommunicationEvent>) -> Self {
    self.sender = Some(sender);
    self
  }

  fn finish(mut self) -> Box<dyn DeviceCommunicationManager> {
    Box::new(WebsocketServerCommunicationManager::new(
      self.sender.take().unwrap(),
      self.server_port,
      self.listen_on_all_interfaces,
    ))
  }
}

pub struct WebsocketServerCommunicationManager {}

impl WebsocketServerCommunicationManager {
  fn new(
    sender: Sender<DeviceCommunicationEvent>,
    port: u16,
    listen_on_all_interfaces: bool,
  ) -> Self {
    trace!("Websocket server port created.");
    tokio::spawn(async move {
      let base_addr = if listen_on_all_interfaces {
        "0.0.0.0"
      } else {
        "127.0.0.1"
      };

      let addr = format!("{}:{}", base_addr, port);
      debug!("Websocket Insecure: Trying to listen on {}", addr);

      // Create the event loop and TCP listener we'll accept connections on.
      let try_socket = TcpListener::bind(&addr).await;
      debug!("Websocket Insecure: Socket bound.");
      let listener = try_socket.unwrap(); //.map_err(|e| ButtplugConnectorError::TransportSpecificError(ButtplugConnectorTransportSpecificError::GenericNetworkError(format!("{:?}", e))))?;
      debug!("Websocket Insecure: Listening on: {}", addr);
      loop {
        let (stream, _) = listener.accept().await.unwrap();
        info!("Websocket Insecure: Got connection");
        let ws_fut = async_tungstenite::tokio::accept_async(stream);
        let mut ws_stream = ws_fut.await.unwrap();
        // Websockets are different from the rest of the communication managers, in that we have no
        // information about the device type when we create the connection, and therefore have to
        // wait for the first packet. We'll have to pass our device event sender off to the newly
        // created event loop, so that it can fire once the info packet is received.
        let sender_clone = sender.clone();
        tokio::spawn(async move {
          // TODO Implement a receive timeout here so we don't wait forever
          if let Some(Ok(async_tungstenite::tungstenite::Message::Text(info_message))) =
            ws_stream.next().await
          {
            let info_packet: WebsocketCommManagerInitInfo =
              serde_json::from_str(&info_message).unwrap();
            if sender_clone
              .send(DeviceCommunicationEvent::DeviceFound {
                name: format!("Websocket Device {}", info_packet.identifier),
                address: info_packet.address.clone(),
                creator: Box::new(WebsocketServerDeviceImplCreator::new(
                  info_packet,
                  ws_stream,
                )),
              })
              .await
              .is_err()
            {
              error!("Device manager disappeared, exiting.");
              return;
            }
          } else {
            error!("Did not receive info message as first packet, dropping connection.");
          }
        });
      }
    });
    Self {}
  }
}

impl DeviceCommunicationManager for WebsocketServerCommunicationManager {
  fn name(&self) -> &'static str {
    "WebsocketServerCommunicationManager"
  }

  fn start_scanning(&self) -> ButtplugResultFuture {
    debug!("Websocket server manager scanning for devices.");
    Box::pin(async move { Ok(()) })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(async move { Ok(()) })
  }
}
