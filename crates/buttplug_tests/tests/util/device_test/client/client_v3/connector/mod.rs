mod in_process_connector;
pub use in_process_connector::{
  ButtplugInProcessClientConnectorBuilder,
};


use buttplug_client::serializer::ButtplugClientJSONSerializer;
use buttplug_core::connector::ButtplugRemoteConnector;
use buttplug_server::message::{ButtplugClientMessageV3, ButtplugServerMessageV3};


pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV3,
  ButtplugServerMessageV3,
>;
