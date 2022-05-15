// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod delay_device_communication_manager;
pub use delay_device_communication_manager::DelayDeviceCommunicationManagerBuilder;
mod channel_transport;
pub use channel_transport::*;
use buttplug::{
  client::{
    ButtplugClient,
  },
  core::{
    connector::ButtplugInProcessClientConnectorBuilder,
  },
  server::{ButtplugServerBuilder, ButtplugServer, device::communication::test::{TestDeviceCommunicationManagerBuilder, TestDeviceInternal}},
};
use std::sync::Arc;

#[allow(dead_code)]
pub fn setup_logging() {
  tracing_subscriber::fmt::init();
}

#[allow(dead_code)]
pub async fn test_client() -> ButtplugClient {
  let mut server_builder = ButtplugServerBuilder::default();
  let connector = ButtplugInProcessClientConnectorBuilder::default().server(server_builder.finish().unwrap()).finish();

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
pub async fn test_client_with_device() -> (ButtplugClient, Arc<TestDeviceInternal>) {
  let builder = TestDeviceCommunicationManagerBuilder::default();
  let helper = builder.helper();

  let mut server_builder = ButtplugServerBuilder::default();
  server_builder.device_manager_builder().comm_manager(builder);
  let connector = ButtplugInProcessClientConnectorBuilder::default().server(server_builder.finish().unwrap()).finish();
  let device = helper.add_ble_device("Massage Demo").await;

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
  server_builder.device_manager_builder().comm_manager(builder);
  let connector = ButtplugInProcessClientConnectorBuilder::default().server(server_builder.finish().unwrap()).finish();

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
pub async fn test_server_with_device(device_type: &str) -> (ButtplugServer, Arc<TestDeviceInternal>) {
  let mut server_builder = ButtplugServerBuilder::default();
  let builder = TestDeviceCommunicationManagerBuilder::default();
  let helper = builder.helper();
  server_builder
    .device_manager_builder()
    .comm_manager(builder);
  let server = server_builder.finish().unwrap();
  let device = helper.add_ble_device(device_type).await;
  (server, device)
}