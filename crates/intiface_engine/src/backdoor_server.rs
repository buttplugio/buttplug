use buttplug_core::{
    connector::transport::stream::ButtplugStreamTransport,
    message::serializer::ButtplugSerializedMessage,
    util::stream::convert_broadcast_receiver_to_stream,
};
use buttplug_server::{connector::ButtplugRemoteServerConnector, device::ServerDeviceManager, message::serializer::ButtplugServerJSONSerializer, ButtplugServerBuilder};
use std::sync::Arc;
use tokio::sync::{
  broadcast,
  mpsc::{self, Sender},
};
use tokio_stream::Stream;

use crate::ButtplugRemoteServer;

// Allows direct access to the Device Manager of a running ButtplugServer. Bypasses requirements for
// client handshake, ping, etc...
pub struct BackdoorServer {
  //server: ButtplugRemoteServer,
  sender: Sender<ButtplugSerializedMessage>,
  broadcaster: broadcast::Sender<String>,
}

impl BackdoorServer {
  pub fn new(device_manager: Arc<ServerDeviceManager>) -> Self {
    let server = ButtplugRemoteServer::new(
      ButtplugServerBuilder::with_shared_device_manager(device_manager.clone())
        .name("Intiface Backdoor Server")
        .finish()
        .unwrap(),
        &None
    );
    let (s_out, mut r_out) = mpsc::channel(255);
    let (s_in, r_in) = mpsc::channel(255);
    let (s_stream, _) = broadcast::channel(255);
    tokio::spawn(async move {
      if let Err(e) = server
        .start(ButtplugRemoteServerConnector::<
          _,
          ButtplugServerJSONSerializer,
        >::new(ButtplugStreamTransport::new(s_out, r_in)))
        .await
      {
        // We can't do much if the server fails, but we *can* yell into the logs!
        error!("Backdoor server error: {:?}", e);
      }
    });
    let sender_clone = s_stream.clone();
    tokio::spawn(async move {
      while let Some(ButtplugSerializedMessage::Text(m)) = r_out.recv().await {
        if sender_clone.receiver_count() == 0 {
          continue;
        }
        if sender_clone.send(m).is_err() {
          break;
        }
      }
    });
    Self {
      sender: s_in,
      broadcaster: s_stream,
    }
  }

  pub fn event_stream(&self) -> impl Stream<Item = String> + '_ {
    convert_broadcast_receiver_to_stream(self.broadcaster.subscribe())
  }

  pub async fn parse_message(&self, msg: &str) {
    self
      .sender
      .send(ButtplugSerializedMessage::Text(msg.to_owned()))
      .await
      .unwrap();
  }
}
