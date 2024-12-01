
#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
mod in_process_connector;

#[cfg(all(feature = "server", feature = "client", not(feature = "wasm")))]
pub use in_process_connector::{
  ButtplugInProcessClientConnector,
  ButtplugInProcessClientConnectorBuilder,
};

use crate::core::connector::ButtplugRemoteConnector;
#[cfg(all(feature = "websockets", feature = "serialize-json"))]
use crate::{core::{
  connector::{ButtplugConnector, ButtplugWebsocketClientTransport},  
  message::{ButtplugClientMessageCurrent, ButtplugServerMessageCurrent}
}, client::serializer::ButtplugClientJSONSerializer};

/// Convenience method for creating a new Buttplug Client Websocket connector that uses the JSON
/// serializer. This is pretty much the only connector used for IPC right now, so this makes it easy
/// to create one without having to fill in the generic types.
#[cfg(all(feature = "websockets", feature = "serialize-json"))]
pub fn new_json_ws_client_connector(
  address: &str,
) -> impl ButtplugConnector<ButtplugClientMessageCurrent, ButtplugServerMessageCurrent> {
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
  ButtplugClientMessageCurrent,
  ButtplugServerMessageCurrent,
>;
