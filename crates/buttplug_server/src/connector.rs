use buttplug_core::connector::ButtplugRemoteConnector;

use super::message::{ButtplugClientMessageVariant, ButtplugServerMessageVariant};

pub type ButtplugRemoteServerConnector<TransportType, SerializerType> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugServerMessageVariant,
  ButtplugClientMessageVariant,
>;
