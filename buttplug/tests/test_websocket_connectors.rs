// The tests in this file can fail on CI if there isn't a timed retry.

#[cfg(all(feature = "websockets", target = "windows"))]
mod websocket_connector_tests {
  use buttplug::{
    client::ButtplugClient,
    connector::{
      ButtplugRemoteClientConnector,
      ButtplugRemoteServerConnector,
      ButtplugWebsocketClientTransport,
      ButtplugWebsocketServerTransport,
      ButtplugWebsocketServerTransportBuilder,
    },
    core::messages::serializer::{ButtplugClientJSONSerializer, ButtplugServerJSONSerializer},
    server::ButtplugRemoteServer,
    util::async_manager,
  };
  use futures_timer::Delay;
  use std::sync::Arc;
  use std::time::Duration;

  #[test]
  fn test_client_ws_client_server_ws_server_insecure() {
    async_manager::block_on(async move {
      let test_server = ButtplugRemoteServer::default();
      let server = Arc::new(test_server);
      let server_clone = server.clone();
      async_manager::spawn(async move {
        let connector = ButtplugRemoteServerConnector::<
          ButtplugWebsocketServerTransport,
          ButtplugServerJSONSerializer,
        >::new(
          ButtplugWebsocketServerTransportBuilder::default()
            .port(12349)
            .finish(),
        );
        server_clone
          .start(connector)
          .await
          .expect("Test, assuming infallible.");
      });
      let mut connected = false;
      for _ in 0..10u8 {
        let connector = ButtplugRemoteClientConnector::<
          ButtplugWebsocketClientTransport,
          ButtplugClientJSONSerializer,
        >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
          "ws://127.0.0.1:12349",
        ));

        let client = ButtplugClient::new("Test Client");
        if client.connect(connector).await.is_ok() {
          connected = true;
          break;
        }
        Delay::new(Duration::from_secs(1)).await;
      }
      assert!(connected);
      server
        .disconnect()
        .await
        .expect("Test, assuming infallible.");
    });
  }

  #[test]
  fn test_client_ws_server_server_ws_client_insecure() {
    async_manager::block_on(async move {
      let test_server = ButtplugRemoteServer::default();
      let server = Arc::new(test_server);
      let server_clone = server.clone();
      async_manager::spawn(async move {
        let connector = ButtplugRemoteServerConnector::<
          ButtplugWebsocketClientTransport,
          ButtplugServerJSONSerializer,
        >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
          "ws://127.0.0.1:12347",
        ));
        server_clone
          .start(connector)
          .await
          .expect("Test, assuming infallible.");
      });

      let mut connected = false;
      for _ in 0..10u8 {
        let connector = ButtplugRemoteClientConnector::<
          ButtplugWebsocketServerTransport,
          ButtplugClientJSONSerializer,
        >::new(
          ButtplugWebsocketServerTransportBuilder::default()
            .port(12347)
            .finish(),
        );

        let client = ButtplugClient::new("Test Client");
        if client.connect(connector).await.is_ok() {
          connected = true;
          break;
        }
        Delay::new(Duration::from_secs(1)).await;
      }
      assert!(connected);
      server
        .disconnect()
        .await
        .expect("Test, assuming infallible.");
    });
  }
}

// TODO Test disconnection event from server side
