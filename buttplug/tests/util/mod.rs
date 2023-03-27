// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod delay_device_communication_manager;
pub mod test_server;
pub use test_server::ButtplugTestServer;
pub mod device_test;
pub use device_test::DeviceTestCase;
pub mod test_device_manager;
pub use delay_device_communication_manager::DelayDeviceCommunicationManagerBuilder;
mod channel_transport;
use buttplug::{
  client::ButtplugClient,
  core::connector::ButtplugInProcessClientConnectorBuilder,
  server::{ButtplugServer, ButtplugServerBuilder},
};
pub use channel_transport::*;
pub use test_device_manager::{
  TestDeviceChannelHost,
  TestDeviceCommunicationManagerBuilder,
  TestHardwareEvent,
  TestHardwareNotification,
};

use crate::util::test_device_manager::TestDeviceIdentifier;

#[allow(dead_code)]
pub fn setup_logging() {
  tracing_subscriber::fmt::init();
}

#[allow(dead_code)]
pub async fn test_client() -> ButtplugClient {
  let mut server_builder = ButtplugServerBuilder::default();
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server_builder.finish().unwrap())
    .finish();

  let client = ButtplugClient::new("Test Client");
  assert!(!client.connected());
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  assert!(client.connected());
  client
}

#[allow(dead_code)]
pub async fn test_client_with_device() -> (ButtplugClient, TestDeviceChannelHost) {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device = builder.add_test_device(&TestDeviceIdentifier::new("Massage Demo", None));

  let mut server_builder = ButtplugServerBuilder::default();
  server_builder.comm_manager(builder);
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server_builder.finish().unwrap())
    .finish();

  let client = ButtplugClient::new("Test Client");
  assert!(!client.connected());
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  assert!(client.connected());
  (client, device)
}

#[allow(dead_code)]
pub async fn test_client_with_delayed_device_manager() -> ButtplugClient {
  let builder = DelayDeviceCommunicationManagerBuilder::default();

  let mut server_builder = ButtplugServerBuilder::default();
  server_builder.comm_manager(builder);
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server_builder.finish().unwrap())
    .finish();

  let client = ButtplugClient::new("Test Client");
  assert!(!client.connected());
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  assert!(client.connected());
  client
}

#[allow(dead_code)]
pub async fn test_server_with_device(
  device_type: &str,
  allow_raw_message: bool,
) -> (ButtplugServer, TestDeviceChannelHost) {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device = builder.add_test_device(&TestDeviceIdentifier::new(device_type, None));

  let mut server_builder = ButtplugServerBuilder::default();
  if allow_raw_message {
    server_builder.allow_raw_messages();
  }
  server_builder.comm_manager(builder);
  let server = server_builder.finish().unwrap();
  (server, device)
}
