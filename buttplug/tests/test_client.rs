mod util;
extern crate buttplug;

use tokio::sync::mpsc::Receiver;
use buttplug::{
  client::{ButtplugClient, ButtplugClientError, ButtplugClientEvent},
  connector::{
    ButtplugConnector,
    ButtplugConnectorError,
    ButtplugConnectorResultFuture,
    ButtplugInProcessClientConnector,
  },
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugServerError},
    messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
  },
  server::ButtplugServerOptions,
  util::async_manager,
};
use futures::{future::BoxFuture, StreamExt};
use futures_timer::Delay;
use std::time::Duration;
use util::DelayDeviceCommunicationManager;

#[derive(Default)]
struct ButtplugFailingConnector {}

impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
  for ButtplugFailingConnector
{
  fn connect(
    &mut self,
  ) -> BoxFuture<
    'static,
    Result<
      Receiver<Result<ButtplugCurrentSpecServerMessage, ButtplugServerError>>,
      ButtplugConnectorError,
    >,
  > {
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
#[test]
fn test_failing_connection() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    assert!(
      client.connect(ButtplugFailingConnector::default())
        .await
        .is_err()
    );
  });
}

#[cfg(feature = "server")]
#[test]
fn test_disconnect_status() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    client.connect(ButtplugInProcessClientConnector::default())
        .await
        .unwrap();
    assert!(client.disconnect().await.is_ok());
    assert!(!client.connected());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_double_disconnect() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    client.connect(ButtplugInProcessClientConnector::default())
        .await
        .unwrap();
    assert!(client.disconnect().await.is_ok());
    assert!(client.disconnect().await.is_err());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_connect_init() {
  async_manager::block_on(async {
    let client = ButtplugClient::new("Test Client");
    client.connect(ButtplugInProcessClientConnector::default())
        .await
        .unwrap();
    assert_eq!(client.server_name(), Some("Buttplug Server".to_owned()));
  });
}

#[cfg(feature = "server")]
#[test]
fn test_start_scanning() {
  async_manager::block_on(async {
    let connector = ButtplugInProcessClientConnector::default();
    let test_mgr_helper = connector.server_ref().add_test_comm_manager().unwrap();
    test_mgr_helper.add_ble_device("Massage Demo").await;
    let client = ButtplugClient::new("Test Client");
    client.connect(connector)
        .await
        .unwrap();
    assert!(client.start_scanning().await.is_ok());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_stop_scanning_when_not_scanning() {
  async_manager::block_on(async {
    let connector = ButtplugInProcessClientConnector::default();
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>()
      .unwrap();
    let client = ButtplugClient::new("Test Client");
    client.connect(connector)
          .await
          .unwrap();
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
  });
}

#[cfg(feature = "server")]
#[test]
fn test_start_scanning_when_already_scanning() {
  async_manager::block_on(async {
    let connector = ButtplugInProcessClientConnector::default();
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>()
      .unwrap();
    let client = ButtplugClient::new("Test Client");
    client.connect(connector)
      .await
      .unwrap();
    assert!(client.start_scanning().await.is_ok());
    assert!(client.start_scanning().await.is_err());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_scanning_finished() {
  async_manager::block_on(async {
    let connector = ButtplugInProcessClientConnector::default();
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>()
      .unwrap();
    let client = ButtplugClient::new("Test Client");
    let mut recv = client.event_stream();
      client.connect(connector)
            .await
            .unwrap();

    assert!(client.start_scanning().await.is_ok());
    assert!(client.stop_scanning().await.is_ok());
    assert!(matches!(
      recv.next().await.unwrap(),
      ButtplugClientEvent::ScanningFinished
    ));
  });
}

#[cfg(feature = "server")]
#[test]
fn test_client_ping() {
  async_manager::block_on(async {
    let mut options = ButtplugServerOptions::default();
    options.max_ping_time = 200;
    let connector = ButtplugInProcessClientConnector::new_with_options(&options).unwrap();
    let client = ButtplugClient::new("Test Client");
      client.connect(connector)
            .await
            .unwrap();
    assert!(client.ping().await.is_ok());
    Delay::new(Duration::from_millis(800)).await;
    assert!(client.ping().await.is_err());
  });
}
