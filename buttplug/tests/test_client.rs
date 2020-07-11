extern crate buttplug;

use async_channel::Receiver;
use buttplug::{
  client::ButtplugClient,
  connector::{
    ButtplugConnector, ButtplugConnectorError, ButtplugConnectorResultFuture,
    ButtplugInProcessClientConnector,
  },
  core::{
    messages::{ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage},
    errors::ButtplugError,
  },
  util::async_manager,
};
use futures::future::BoxFuture;

#[derive(Default)]
struct ButtplugFailingConnector {}

impl ButtplugConnector<ButtplugCurrentSpecClientMessage, ButtplugCurrentSpecServerMessage>
  for ButtplugFailingConnector
{
  fn connect(
    &mut self,
  ) -> BoxFuture<'static, Result<Receiver<Result<ButtplugCurrentSpecServerMessage, ButtplugError>>, ButtplugConnectorError>>
  {
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
#[test]
#[ignore]
fn test_start_scanning() {
  async_manager::block_on(async {
    let (client, _) = ButtplugClient::connect(
      "Test Client",
      ButtplugInProcessClientConnector::new("Test Server", 0),
    )
    .await
    .unwrap();
    assert!(client.start_scanning().await.is_ok());
  });
}

#[test]
fn test_stop_scanning_when_not_scanning() {
  async_manager::block_on(async {
    let mut connector = ButtplugInProcessClientConnector::new("Test Server", 0);
    connector.server_ref().add_test_comm_manager();
    let (client, _) = ButtplugClient::connect("Test Client", connector)
      .await
      .unwrap();
    assert!(client.stop_scanning().await.is_err());
    assert!(client.stop_scanning().await.is_err());
  });
}

// #[test]
// fn test_scanning_finished() {
//     task::block_on(async {
//         let mut client = connect_test_client().await;
//         assert_eq!(client.server_name.as_ref().unwrap(), "Test Server");
//         assert!(client.start_scanning().await.is_none());
//     });
// }

// Failure on server version error is unit tested in server.
