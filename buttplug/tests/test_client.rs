// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;
use util::{test_client, test_client_with_delayed_device_manager, test_client_with_device};
extern crate buttplug;
extern crate tracing;

use buttplug::{
  client::{ButtplugClient, ButtplugClientError, ButtplugClientEvent},
  core::{
    connector::{
      ButtplugConnector,
      ButtplugConnectorError,
      ButtplugConnectorResultFuture,
      ButtplugInProcessClientConnectorBuilder,
    },
    errors::{ButtplugDeviceError, ButtplugError},
    message::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
  },
  server::ButtplugServerBuilder,
};

use futures::{future::BoxFuture, StreamExt};
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time::sleep};

#[derive(Default)]
struct ButtplugFailingConnector {}

impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
  for ButtplugFailingConnector
{
  fn connect(
    &mut self,
    _: Sender<ButtplugCurrentSpecServerMessage>,
  ) -> BoxFuture<'static, Result<(), ButtplugConnectorError>> {
    ButtplugConnectorError::ConnectorNotConnected.into()
  }

  fn disconnect(&self) -> ButtplugConnectorResultFuture {
    ButtplugConnectorError::ConnectorNotConnected.into()
  }

  fn send(&self, _msg: ButtplugCurrentSpecClientMessage) -> ButtplugConnectorResultFuture {
    panic!("Should never be called")
  }
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_failing_connection() {
  let client = ButtplugClient::new("Test Client");
  assert!(client
    .connect(ButtplugFailingConnector::default())
    .await
    .is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_disconnect_status() {
  let client = test_client().await;
  assert!(client.disconnect().await.is_ok());
  assert!(!client.connected());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_double_disconnect() {
  let client = test_client().await;
  assert!(client.disconnect().await.is_ok());
  assert!(client.disconnect().await.is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_connect_init() {
  let client = test_client().await;
  assert_eq!(client.server_name(), Some("Buttplug Server".to_owned()));
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_client_connected_status() {
  let client = test_client().await;
  client
    .disconnect()
    .await
    .expect("Test, assuming infallible.");
  assert!(!client.connected());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_start_scanning() {
  let (client, _) = test_client_with_device().await;
  assert!(client.start_scanning().await.is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
#[ignore = "We may want to just call this Ok now?"]
async fn test_stop_scanning_when_not_scanning() {
  let (client, _) = test_client_with_device().await;
  let should_be_err = client.stop_scanning().await;
  if let Err(ButtplugClientError::ButtplugError(bp_err)) = should_be_err {
    assert!(matches!(
      bp_err,
      ButtplugError::ButtplugDeviceError(ButtplugDeviceError::DeviceScanningAlreadyStopped)
    ));
  } else {
    panic!("Should've thrown error!");
  }
  assert!(client.stop_scanning().await.is_err());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_start_scanning_when_already_scanning() {
  let client = test_client_with_delayed_device_manager().await;
  assert!(client.start_scanning().await.is_ok());
  assert!(client.start_scanning().await.is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_successive_start_scanning() {
  let (client, _) = test_client_with_device().await;
  assert!(client.start_scanning().await.is_ok());
  assert!(client.start_scanning().await.is_ok());
}

#[cfg(feature = "server")]
#[tokio::test]
async fn test_client_scanning_finished() {
  let (client, _) = test_client_with_device().await;
  let mut recv = client.event_stream();
  assert!(client.start_scanning().await.is_ok());
  assert!(matches!(
    recv.next().await.expect("Test, assuming infallible."),
    ButtplugClientEvent::ScanningFinished
  ));
}

#[cfg(feature = "server")]
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
  // TODO Watch for ping events
  assert!(client.ping().await.is_err());
}
/*
// Tests both the stop all devices functionality, as well as both ends of the
// command range for is_in_command_range message validation.
#[cfg(feature = "server")]
#[tokio::test]
async fn test_stop_all_devices_and_device_command_range() {
    let (client, test_device) = test_client_with_device().await;
    let mut event_stream = client.event_stream();
    assert!(client.start_scanning().await.is_ok());

    while let Some(event) = event_stream.next().await {
      if let ButtplugClientEvent::DeviceAdded(dev) = event {
        info!("{:?}", dev.vibrate(ScalarCommand::Scalar(0.5)).await);
        assert!(dev.vibrate(ScalarCommand::Scalar(0.5)).await.is_ok());
        // Unlike protocol unit tests, here the endpoint doesn't exist until
        // after device creation, so create the test receiver later.
        let command_receiver = test_device
          .endpoint_receiver(&Endpoint::Tx)
          .expect("Test, assuming infallible.");
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 64], false)),
        );
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 64], false)),
        );
        assert!(dev.vibrate(ScalarCommand::Scalar(1.0)).await.is_ok());
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 127], false)),
        );
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 127], false)),
        );
        assert!(client.stop_all_devices().await.is_ok());
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF1, 0], false)),
        );
        check_test_recv_value(
          &command_receiver,
          HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0xF2, 0], false)),
        );
        break;
      }
    }
    assert!(client.stop_all_devices().await.is_ok());
}
*/
// TODO Test calling connect twice
// TODO Test calling disconnect twice w/o connection
// TODO Test invalid return on RequestServerInfo
// TODO Test invalid return on DeviceList
// TODO Test receiving unmatched Ok (should emit error)
// TODO Test receiving unmatched DeviceRemoved
// TODO Test receiving Error when expecting Ok (i.e. StartScanning returns an error)
// TODO Test receiving wrong message expecting Ok (i.e. StartScanning returns DeviceList)
