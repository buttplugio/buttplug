use buttplug::{
  client::ButtplugClient,
  core::connector::ButtplugInProcessClientConnectorBuilder,
  server::{
    device::hardware::communication::btleplug::BtlePlugCommunicationManagerBuilder,
    ButtplugServerBuilder,
  },
  util::in_process_client,
};

#[allow(dead_code)]
async fn main_the_hard_way() -> anyhow::Result<()> {
  let mut server_builder = ButtplugServerBuilder::default();
  // This is how we add Bluetooth manually. (We could also do this with any other communication manager.)
  server_builder.comm_manager(BtlePlugCommunicationManagerBuilder::default());
  let server = server_builder.finish().unwrap();

  // First off, we'll set up our Embedded Connector.
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();

  let client = ButtplugClient::new("Example Client");
  client.connect(connector).await?;

  Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // This is the easy way, it sets up an embedded server with everything set up automatically
  let _client = in_process_client("Example Client", false).await;

  Ok(())
}
