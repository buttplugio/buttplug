use super::serializer::ButtplugClientJSONSerializer;
use buttplug_core::{
  connector::ButtplugRemoteConnector,
  message::{ButtplugClientMessageV4, ButtplugServerMessageV4},
};

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV4,
  ButtplugServerMessageV4,
>;
