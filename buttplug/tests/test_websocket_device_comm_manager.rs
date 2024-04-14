// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

#[cfg(feature = "websocket-server-manager")]
mod test {

  use buttplug::{
    client::ButtplugClient,
    core::connector::ButtplugInProcessClientConnectorBuilder,
    server::device::hardware::communication::websocket_server::websocket_server_comm_manager::WebsocketServerDeviceCommunicationManagerBuilder,
  };

use crate::util::test_server_with_comm_manager;

  async fn setup_test_client() -> ButtplugClient {
    let server = test_server_with_comm_manager( WebsocketServerDeviceCommunicationManagerBuilder::default()
    .server_port(51283)
    .listen_on_all_interfaces(true), false);
    let connector = ButtplugInProcessClientConnectorBuilder::default()
      .server(server)
      .finish();

    let client = ButtplugClient::new("Websocket DCM Test Client");
    client
      .connect(connector)
      .await
      .expect("Test, assuming infallible.");
    client
  }

  #[tokio::test]
  async fn test_websocket_server_dcm_bringup() {
    let client = setup_test_client().await;
    assert!(client.connected());
  }
}
