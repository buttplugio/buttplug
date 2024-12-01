#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
mod in_process_connector;

#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
pub use in_process_connector::{
  ButtplugInProcessClientConnector, ButtplugInProcessClientConnectorBuilder,
};

#[cfg(all(feature = "websockets", feature = "serialize-json"))]
use crate::{
  client::serializer::ButtplugClientJSONSerializer,
  core::connector::{ButtplugConnector, ButtplugWebsocketClientTransport},
};
use crate::{
  core::connector::ButtplugRemoteConnector,
  server::message::{ButtplugClientMessageV3, ButtplugServerMessageV3},
};

/// Convenience method for creating a new Buttplug Client Websocket connector that uses the JSON
/// serializer. This is pretty much the only connector used for IPC right now, so this makes it easy
/// to create one without having to fill in the generic types.
#[cfg(all(feature = "websockets", feature = "serialize-json"))]
pub fn new_json_ws_client_connector(
  address: &str,
) -> impl ButtplugConnector<ButtplugClientMessageV3, ButtplugServerMessageV3> {
  ButtplugRemoteClientConnector::<
      ButtplugWebsocketClientTransport,
      ButtplugClientJSONSerializer,
    >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    address,
  ))
}

pub type ButtplugRemoteClientConnector<
  TransportType,
  SerializerType = ButtplugClientJSONSerializer,
> = ButtplugRemoteConnector<
  TransportType,
  SerializerType,
  ButtplugClientMessageV3,
  ButtplugServerMessageV3,
>;
