#[cfg(all(feature = "websockets", feature = "async-std-runtime"))]
mod websocket_connector_tests {
  use buttplug::{
    client::ButtplugClient,
    connector::{
      ButtplugRemoteClientConnector,
      ButtplugRemoteServerConnector,
      ButtplugWebsocketClientTransport,
      ButtplugWebsocketServerTransport,
      ButtplugWebsocketServerTransportOptions,
    },
    core::messages::serializer::{ButtplugClientJSONSerializer, ButtplugServerJSONSerializer},
    server::ButtplugRemoteServer,
    util::async_manager,
  };
  use std::sync::Arc;

  #[test]
  fn test_client_ws_client_server_ws_server_insecure() {
    async_manager::block_on(async move {
      let (test_server, _) = ButtplugRemoteServer::default();
      let server = Arc::new(test_server);
      let server_clone = server.clone();
      async_manager::spawn(async move {
        let connector = ButtplugRemoteServerConnector::<
          ButtplugWebsocketServerTransport,
          ButtplugServerJSONSerializer,
        >::new(ButtplugWebsocketServerTransport::new(
          ButtplugWebsocketServerTransportOptions {
            ws_listen_on_all_interfaces: false,
            ws_insecure_port: Some(12345u16),
            ws_secure_port: None,
            ws_cert_file: None,
            ws_priv_file: None,
          },
        ));
        server_clone.start(connector).await.unwrap();
      })
      .unwrap();

      let connector = ButtplugRemoteClientConnector::<
        ButtplugWebsocketClientTransport,
        ButtplugClientJSONSerializer,
      >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
        "ws://127.0.0.1:12345",
      ));

      let (client, _) = ButtplugClient::connect("Example Client", connector)
        .await
        .unwrap();
      assert!(client.connected());
      server.disconnect().await.unwrap();
    });
  }

  #[test]
  fn test_client_ws_server_server_ws_client_insecure() {
    async_manager::block_on(async move {
      let (test_server, _) = ButtplugRemoteServer::default();
      let server = Arc::new(test_server);
      let server_clone = server.clone();
      async_manager::spawn(async move {
        let connector = ButtplugRemoteServerConnector::<
          ButtplugWebsocketClientTransport,
          ButtplugServerJSONSerializer,
        >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
          "ws://127.0.0.1:12347",
        ));
        server_clone.start(connector).await.unwrap();
      })
      .unwrap();

      let connector = ButtplugRemoteClientConnector::<
        ButtplugWebsocketServerTransport,
        ButtplugClientJSONSerializer,
      >::new(ButtplugWebsocketServerTransport::new(
        ButtplugWebsocketServerTransportOptions {
          ws_listen_on_all_interfaces: false,
          ws_insecure_port: Some(12347u16),
          ws_secure_port: None,
          ws_cert_file: None,
          ws_priv_file: None,
        },
      ));

      let (client, _) = ButtplugClient::connect("Example Client", connector)
        .await
        .unwrap();
      assert!(client.connected());
      server.disconnect().await.unwrap();
    });
  }

  // fn test_client_ws_client_server_ws_both() {}
}
