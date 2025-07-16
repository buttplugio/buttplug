use buttplug::{
  client::ButtplugClient,
  core::{
    connector::{ButtplugRemoteConnector, ButtplugWebsocketClientTransport, new_json_ws_client_connector},
    message::serializer::ButtplugClientJSONSerializer,
  },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // To create a Websocket Connector, you need the websocket address and some generics fuckery.
  let connector = new_json_ws_client_connector("ws://192.168.123.103:12345/buttplug");

  let client = ButtplugClient::new("Example Client");
  client.connect(connector).await?;

  Ok(())
}
