#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
mod in_process_connector;

#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
pub use in_process_connector::{
  ButtplugInProcessClientConnector,
  ButtplugInProcessClientConnectorBuilder,
};

use crate::{
  client::serializer::ButtplugClientJSONSerializer,
  core::{
    connector::ButtplugRemoteConnector,
    message::{ButtplugClientMessageV4, ButtplugServerMessageV4},
  }
};
#[cfg(feature = "websockets")]
use crate::{
  core::connector::{ButtplugConnector, ButtplugWebsocketClientTransport},
};

/// Convenience method for creating a new Buttplug Client Websocket connector that uses the JSON
/// serializer. This is pretty much the only connector used for IPC right now, so this makes it easy
/// to create one without having to fill in the generic types.
#[cfg(feature = "websockets")]
pub fn new_json_ws_client_connector(
  address: &str,
) -> impl ButtplugConnector<ButtplugClientMessageV4, ButtplugServerMessageV4> {
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
  ButtplugClientMessageV4,
  ButtplugServerMessageV4,
>;
