#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
mod in_process_connector;

#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
pub use in_process_connector::{
  ButtplugInProcessClientConnectorBuilder,
};

use buttplug::{
  client::serializer::ButtplugClientJSONSerializer,
  core::connector::ButtplugRemoteConnector,
  server::message::{ButtplugClientMessageV3, ButtplugServerMessageV3},
};

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV3,
  ButtplugServerMessageV3,
>;
