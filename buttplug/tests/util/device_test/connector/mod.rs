pub mod channel_transport;
use buttplug::core::{
  connector::{ButtplugRemoteClientConnector, ButtplugRemoteServerConnector, ButtplugRemoteConnector},
  messages::{
    serializer::{ButtplugClientJSONSerializer, ButtplugServerJSONSerializer},
    ButtplugSpecV2ClientMessage, ButtplugSpecV2ServerMessage,
    ButtplugSpecV1ClientMessage, ButtplugSpecV1ServerMessage,
    ButtplugSpecV0ClientMessage, ButtplugSpecV0ServerMessage,        
  },
};
use tokio::sync::{mpsc, Notify};
use std::sync::Arc;

use self::channel_transport::ChannelTransport;

pub type ChannelClientConnectorCurrent =
  ButtplugRemoteClientConnector<channel_transport::ChannelTransport, ButtplugClientJSONSerializer>;

pub type ChannelClientConnectorV2 = ButtplugRemoteConnector<
  channel_transport::ChannelTransport,
  ButtplugClientJSONSerializer,
  ButtplugSpecV2ClientMessage,
  ButtplugSpecV2ServerMessage,
>;

pub type ChannelClientConnectorV1 = ButtplugRemoteConnector<
  channel_transport::ChannelTransport,
  ButtplugClientJSONSerializer,
  ButtplugSpecV1ClientMessage,
  ButtplugSpecV1ServerMessage,
>;

pub type ChannelClientConnectorV0 = ButtplugRemoteConnector<
  channel_transport::ChannelTransport,
  ButtplugClientJSONSerializer,
  ButtplugSpecV0ClientMessage,
  ButtplugSpecV0ServerMessage,
>;

pub type ChannelServerConnector =
  ButtplugRemoteServerConnector<channel_transport::ChannelTransport, ButtplugServerJSONSerializer>;

pub fn build_channel_connector(notify: &Arc<Notify>) -> (ChannelClientConnectorCurrent, ChannelServerConnector) {
  let (server_sender, server_receiver) = mpsc::channel(256);
  let (client_sender, client_receiver) = mpsc::channel(256);

  let client_connector = ChannelClientConnectorCurrent::new(ChannelTransport::new(notify, server_sender, client_receiver));
  let server_connector = ChannelServerConnector::new(ChannelTransport::new(notify, client_sender, server_receiver));
  (client_connector, server_connector)
}