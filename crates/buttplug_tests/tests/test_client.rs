// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_client::{
  ButtplugClient,
  ButtplugClientEvent,
  device::{ClientDeviceCommandValue, ClientDeviceOutputCommand},
};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_core::{
  connector::{ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture},
  message::{ButtplugClientMessageV4, ButtplugServerMessageV4},
};
use buttplug_server::{
  ButtplugServerBuilder,
  device::hardware::{HardwareCommand, HardwareWriteCmd},
};
use buttplug_server_device_config::Endpoint;
use futures::{StreamExt, future::BoxFuture};
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::sleep};
use util::{
  test_client,
  test_client_with_delayed_device_manager,
  test_client_with_device,
  test_device_manager::check_test_recv_value,
};
use uuid::Uuid;

#[derive(Default)]
struct ButtplugFailingConnector {}

impl ButtplugConnector<ButtplugClientMessageV4, ButtplugServerMessageV4>
  for ButtplugFailingConnector
{
  fn connect(
    &mut self,
    _: Sender<ButtplugServerMessageV4>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    ButtplugConnectorError::ConnectorNotConnected.into()
  }

  fn disconnect(&self) -> ButtplugConnectorResultFuture {
    ButtplugConnectorError::ConnectorNotConnected.into()
  }

  fn send(&self, _msg: ButtplugClientMessageV4) -> ButtplugConnectorResultFuture {
    panic!("Should never be called")
  }
}

async fn wait_for_device_added(client: &ButtplugClient) -> buttplug_client::ButtplugClientDevice {
  let mut event_stream = client.event_stream();
  client
    .start_scanning()
    .await
    .expect("Test, assuming infallible.");
  while let Some(event) = event_stream.next().await {
    if let ButtplugClientEvent::DeviceAdded(device) = event {
      return device;
    }
  }
  panic!("DeviceAdded event stream ended unexpectedly");
}

#[tokio::test]
async fn test_failing_connection() {
  let client = ButtplugClient::new("Test Client");
  assert!(
    client
      .connect(ButtplugFailingConnector::default())
      .await
      .is_err()
  );
}

#[tokio::test]
async fn test_disconnect_status() {
  let client = test_client().await;
  assert!(client.disconnect().await.is_ok());
  assert!(!client.connected());
}

#[tokio::test]
async fn test_double_disconnect() {
  let client = test_client().await;
  assert!(client.disconnect().await.is_ok());
  assert!(client.disconnect().await.is_err());
}

#[tokio::test]
async fn test_connect_init() {
  let client = test_client().await;
  assert_eq!(client.server_name(), Some("Buttplug Server".to_owned()));
}

#[tokio::test]
async fn test_client_connected_status() {
  let client = test_client().await;
  client
    .disconnect()
    .await
    .expect("Test, assuming infallible.");
  assert!(!client.connected());
}

#[tokio::test]
async fn test_start_scanning() {
  let (client, _) = test_client_with_device().await;
  assert!(client.start_scanning().await.is_ok());
}

#[tokio::test]
async fn test_stop_scanning_when_not_scanning() {
  let (client, _) = test_client_with_device().await;
  assert!(client.stop_scanning().await.is_ok());
  assert!(client.stop_scanning().await.is_ok());
}

#[tokio::test]
async fn test_start_scanning_when_already_scanning() {
  let client = test_client_with_delayed_device_manager().await;
  assert!(client.start_scanning().await.is_ok());
  assert!(client.start_scanning().await.is_ok());
}

#[tokio::test]
async fn test_successive_start_scanning() {
  let (client, _) = test_client_with_device().await;
  assert!(client.start_scanning().await.is_ok());
  assert!(client.start_scanning().await.is_ok());
}

#[tokio::test]
async fn test_client_scanning_finished() {
  let (client, _) = test_client_with_device().await;
  let mut recv = client.event_stream();
  assert!(client.start_scanning().await.is_ok());
  assert!(matches!(
    recv.next().await.expect("Test, assuming infallible."),
    ButtplugClientEvent::DeviceListReceived
  ));
  assert!(matches!(
    recv.next().await.expect("Test, assuming infallible."),
    ButtplugClientEvent::ScanningFinished
  ));
}

#[tokio::test]
async fn test_client_ping() {
  let server = ButtplugServerBuilder::default()
    .max_ping_time(200)
    .finish()
    .expect("Test, assuming infallible.");
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();
  let client = ButtplugClient::new("Test Client");
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  assert!(client.ping().await.is_ok());
  sleep(Duration::from_millis(800)).await;
  assert!(client.ping().await.is_err());
}

// Tests both the stop-all-devices functionality and the low/high ends of the
// client command range conversion.
#[tokio::test]
async fn test_stop_all_devices_and_device_command_range() {
  let (client, mut test_device) = test_client_with_device().await;
  let dev = wait_for_device_added(&client).await;

  assert!(
    dev
      .run_output(&ClientDeviceOutputCommand::Vibrate(
        ClientDeviceCommandValue::Percent(0.5),
      ))
      .await
      .is_ok()
  );
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF1, 64],
      false,
    )),
  )
  .await;
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF2, 64],
      false,
    )),
  )
  .await;

  assert!(
    dev
      .run_output(&ClientDeviceOutputCommand::Vibrate(
        ClientDeviceCommandValue::Percent(1.0),
      ))
      .await
      .is_ok()
  );
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF1, 127],
      false,
    )),
  )
  .await;
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF2, 127],
      false,
    )),
  )
  .await;

  assert!(client.stop_all_devices().await.is_ok());
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF1, 0],
      false,
    )),
  )
  .await;
  check_test_recv_value(
    &Duration::from_millis(150),
    &mut test_device,
    HardwareCommand::Write(HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0xF2, 0],
      false,
    )),
  )
  .await;

  assert!(client.stop_all_devices().await.is_ok());
}

// TODO Test calling connect twice
// TODO Test invalid return on RequestServerInfo
// TODO Test invalid return on DeviceList
// TODO Test receiving unmatched Ok (should emit error)
// TODO Test receiving unmatched DeviceRemoved
// TODO Test receiving Error when expecting Ok (i.e. StartScanning returns an error)
// TODO Test receiving wrong message expecting Ok (i.e. StartScanning returns DeviceList)
