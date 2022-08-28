pub mod channel_transport;
use buttplug::core::{
  connector::{
    ButtplugRemoteClientConnector,
    ButtplugRemoteConnector,
    ButtplugRemoteServerConnector,
  },
  message::{
    serializer::{
      ButtplugClientJSONSerializer,
      ButtplugClientJSONSerializerImpl,
      ButtplugMessageSerializer,
      ButtplugSerializedMessage,
      ButtplugSerializerError,
      ButtplugServerJSONSerializer,
    },
    ButtplugSpecV0ClientMessage,
    ButtplugSpecV0ServerMessage,
    ButtplugSpecV1ClientMessage,
    ButtplugSpecV1ServerMessage,
    ButtplugSpecV2ClientMessage,
    ButtplugSpecV2ServerMessage,
  },
};
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

use self::channel_transport::ChannelTransport;

#[derive(Default)]
pub struct ButtplugClientJSONSerializerV2 {
  serializer_impl: ButtplugClientJSONSerializerImpl,
}

impl ButtplugMessageSerializer for ButtplugClientJSONSerializerV2 {
  type Inbound = ButtplugSpecV2ServerMessage;
  type Outbound = ButtplugSpecV2ClientMessage;

  fn deserialize(
    &self,
    msg: &ButtplugSerializedMessage,
  ) -> Result<Vec<Self::Inbound>, ButtplugSerializerError> {
    self.serializer_impl.deserialize(msg)
  }

  fn serialize(&self, msg: &[Self::Outbound]) -> ButtplugSerializedMessage {
    self.serializer_impl.serialize(msg)
  }
}

pub type ChannelClientConnectorCurrent =
  ButtplugRemoteClientConnector<channel_transport::ChannelTransport, ButtplugClientJSONSerializer>;

pub type ChannelClientConnectorV2 = ButtplugRemoteConnector<
  channel_transport::ChannelTransport,
  ButtplugClientJSONSerializerV2,
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

pub fn build_channel_connector(
  notify: &Arc<Notify>,
) -> (ChannelClientConnectorCurrent, ChannelServerConnector) {
  let (server_sender, server_receiver) = mpsc::channel(256);
  let (client_sender, client_receiver) = mpsc::channel(256);

  let client_connector = ChannelClientConnectorCurrent::new(ChannelTransport::new(
    notify,
    server_sender,
    client_receiver,
  ));
  let server_connector = ChannelServerConnector::new(ChannelTransport::new(
    notify,
    client_sender,
    server_receiver,
  ));
  (client_connector, server_connector)
}

pub fn build_channel_connector_v2(
  notify: &Arc<Notify>,
) -> (ChannelClientConnectorV2, ChannelServerConnector) {
  let (server_sender, server_receiver) = mpsc::channel(256);
  let (client_sender, client_receiver) = mpsc::channel(256);

  let client_connector = ChannelClientConnectorV2::new(ChannelTransport::new(
    notify,
    server_sender,
    client_receiver,
  ));
  let server_connector = ChannelServerConnector::new(ChannelTransport::new(
    notify,
    client_sender,
    server_receiver,
  ));
  (client_connector, server_connector)
}
