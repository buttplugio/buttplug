mod util;

use buttplug::{
  client::ButtplugClient,
  connector::ButtplugInProcessClientConnector,
  server::comm_managers::websocket_server::websocket_server_comm_manager::WebsocketServerDeviceCommunicationManagerBuilder,
  server::ButtplugServerBuilder,
  util::async_manager,
};

async fn setup_test_client() -> ButtplugClient {
  let server = ButtplugServerBuilder::default().name("Websocket DCM Test Server").finish().unwrap();
  server
  .device_manager()
  .add_comm_manager(
    WebsocketServerDeviceCommunicationManagerBuilder::default().server_port(51283).listen_on_all_interfaces(true),
  )
  .unwrap();
  let connector = ButtplugInProcessClientConnector::new(Some(server));


  let client = ButtplugClient::new("Websocket DCM Test Client");
  client.connect(connector).await.unwrap();
  client
}

#[test]
fn test_websocket_server_dcm_bringup() {
  async_manager::block_on(async {
    let client = setup_test_client().await;
    assert!(client.connected());
  });
}
