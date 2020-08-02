mod util;
extern crate buttplug;

use async_channel::Receiver;
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
  util::async_manager,
};
use futures::{future::BoxFuture, StreamExt};
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
    assert!(
      ButtplugClient::connect("Test Client", ButtplugFailingConnector::default())
        .await
        .is_err()
    );
  });
}

#[cfg(feature = "server")]
#[test]
fn test_disconnect_status() {
  async_manager::block_on(async {
    let (client, _) = ButtplugClient::connect(
      "Test Client",
      ButtplugInProcessClientConnector::new("Test Server", 0),
    )
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
    let (client, _) = ButtplugClient::connect(
      "Test Client",
      ButtplugInProcessClientConnector::new("Test Server", 0),
    )
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
    let (client, _) = ButtplugClient::connect(
      "Test Client",
      ButtplugInProcessClientConnector::new("Test Server", 0),
    )
    .await
    .unwrap();
    assert_eq!(client.server_name, "Test Server");
  });
}

// Test ignored until we have a test device manager.
#[cfg(feature = "server")]
#[test]
fn test_start_scanning() {
  async_manager::block_on(async {
    let mut connector = ButtplugInProcessClientConnector::new("Test Server", 0);
    let test_mgr_helper = connector.server_ref().add_test_comm_manager();
    test_mgr_helper.add_ble_device("Massage Demo").await;
    let (client, _) = ButtplugClient::connect("Test Client", connector)
      .await
      .unwrap();
    assert!(client.start_scanning().await.is_ok());
  });
}

#[cfg(feature = "server")]
#[test]
fn test_stop_scanning_when_not_scanning() {
  async_manager::block_on(async {
    let mut connector = ButtplugInProcessClientConnector::new("Test Server", 0);
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>();
    let (client, _) = ButtplugClient::connect("Test Client", connector)
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
    let mut connector = ButtplugInProcessClientConnector::new("Test Server", 0);
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>();
    let (client, _) = ButtplugClient::connect("Test Client", connector)
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
    let mut connector = ButtplugInProcessClientConnector::new("Test Server", 0);
    connector
      .server_ref()
      .add_comm_manager::<DelayDeviceCommunicationManager>();
    let (client, mut recv) = ButtplugClient::connect("Test Client", connector)
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
