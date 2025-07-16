use buttplug_client::ButtplugClient;
use buttplug_server::{device::ServerDeviceManagerBuilder, ButtplugServerBuilder};
use buttplug_server_device_config::DeviceConfigurationManagerBuilder;
use buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder;
use buttplug_client_in_process::{ButtplugInProcessClientConnectorBuilder, in_process_client};

#[allow(dead_code)]
async fn main_the_hard_way() -> anyhow::Result<()> {
  let dcm = DeviceConfigurationManagerBuilder::default()
    .finish()
    .unwrap();

  let mut device_manager_builder = ServerDeviceManagerBuilder::new(dcm);
  device_manager_builder.comm_manager(BtlePlugCommunicationManagerBuilder::default());

  // This is how we add Bluetooth manually. (We could also do this with any other communication manager.)
  
  let server = ButtplugServerBuilder::new(device_manager_builder.finish().unwrap()).finish().unwrap();

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
  let _client = in_process_client("Example Client").await;

  Ok(())
}
