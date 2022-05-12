// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug::{
  client::ButtplugClient,
  core::connector::ButtplugInProcessClientConnector,
  server::device::communication_manager::websocket_server::websocket_server_comm_manager::WebsocketServerDeviceCommunicationManagerBuilder,
  server::ButtplugServerBuilder,
  util::async_manager,
};

async fn setup_test_client() -> ButtplugClient {
  let server = ButtplugServerBuilder::default()
    .name("Websocket DCM Test Server")
    .finish()
    .expect("Test, assuming infallible.");
  server
    .device_manager()
    .add_comm_manager(
      WebsocketServerDeviceCommunicationManagerBuilder::default()
        .server_port(51283)
        .listen_on_all_interfaces(true),
    )
    .expect("Test, assuming infallible.");
  let connector = ButtplugInProcessClientConnector::new(Some(server));

  let client = ButtplugClient::new("Websocket DCM Test Client");
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  client
}

#[test]
fn test_websocket_server_dcm_bringup() {
  async_manager::block_on(async {
    let client = setup_test_client().await;
    assert!(client.connected());
  });
}
