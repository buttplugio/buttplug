// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod delay_device_communication_manager;
pub mod test_server;
pub use test_server::ButtplugTestServer;
pub mod device_test;
pub mod test_device_manager;
pub use delay_device_communication_manager::DelayDeviceCommunicationManagerBuilder;
pub mod channel_transport;
use buttplug_client::ButtplugClient;
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_server_device_config::{load_protocol_configs, DeviceConfigurationManager};
use buttplug_server::{
    device::{
      hardware::communication::HardwareCommunicationManagerBuilder,
      ServerDeviceManagerBuilder,
    },
    ButtplugServer,
    ButtplugServerBuilder,
};
pub use test_device_manager::{
  TestDeviceChannelHost,
  TestDeviceCommunicationManagerBuilder,
  TestHardwareEvent,
};

use crate::util::test_device_manager::TestDeviceIdentifier;

pub fn create_test_dcm() -> DeviceConfigurationManager {
  load_protocol_configs(&None, &None, false)
    .expect("If this fails, the whole library goes with it.")
    .finish()
    .expect("If this fails, the whole library goes with it.")
}

#[allow(dead_code)]
pub fn setup_logging() {
  tracing_subscriber::fmt::init();
}

#[allow(dead_code)]
pub fn test_server() -> ButtplugServer {
  ButtplugServerBuilder::new(
    ServerDeviceManagerBuilder::new(create_test_dcm())
      .finish()
      .unwrap(),
  )
  .finish()
  .unwrap()
}

#[allow(dead_code)]
pub async fn test_client() -> ButtplugClient {
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(test_server())
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

  let mut dm_builder = ServerDeviceManagerBuilder::new(create_test_dcm());
  dm_builder.comm_manager(builder);

  let server_builder = ButtplugServerBuilder::new(dm_builder.finish().unwrap());

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
pub async fn test_client_with_device_and_custom_dcm(
  identifier: &TestDeviceIdentifier,
  dcm: DeviceConfigurationManager,
) -> (ButtplugClient, TestDeviceChannelHost) {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device = builder.add_test_device(identifier);

  let mut dm_builder = ServerDeviceManagerBuilder::new(dcm);
  dm_builder.comm_manager(builder);

  let server_builder = ButtplugServerBuilder::new(dm_builder.finish().unwrap());

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

  let mut dm_builder = ServerDeviceManagerBuilder::new(create_test_dcm());
  dm_builder.comm_manager(builder);

  let server_builder = ButtplugServerBuilder::new(dm_builder.finish().unwrap());

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
pub fn test_server_with_comm_manager<T>(dcm: T) -> ButtplugServer
where
  T: HardwareCommunicationManagerBuilder + 'static,
{
  let mut dm_builder = ServerDeviceManagerBuilder::new(create_test_dcm());
  dm_builder.comm_manager(dcm);

  ButtplugServerBuilder::new(dm_builder.finish().unwrap())
    .finish()
    .unwrap()
}

#[allow(dead_code)]
pub fn test_server_with_device(
  device_type: &str,
) -> (ButtplugServer, TestDeviceChannelHost) {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device = builder.add_test_device(&TestDeviceIdentifier::new(device_type, None));

  (
    test_server_with_comm_manager(builder),
    device,
  )
}

#[allow(dead_code)]
pub fn test_server_v4_with_device(
  device_type: &str,
) -> (ButtplugServer, TestDeviceChannelHost) {
  let mut builder = TestDeviceCommunicationManagerBuilder::default();
  let device = builder.add_test_device(&TestDeviceIdentifier::new(device_type, None));

  (
    test_server_with_comm_manager(builder),
    device,
  )
}
