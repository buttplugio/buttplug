use buttplug_client::{connector::ButtplugRemoteClientConnector, serializer::ButtplugClientJSONSerializer, ButtplugClient, ButtplugClientEvent};
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketClientTransport;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // To create a Websocket Connector, you need the websocket address and some generics fuckery.
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://127.0.0.1:12345",
  ));
  let client = ButtplugClient::new("Example Client");
  client.connect(connector).await?;

  Ok(())
}
