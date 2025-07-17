use buttplug_client::{connector::ButtplugRemoteClientConnector, serializer::ButtplugClientJSONSerializer, ButtplugClient, ButtplugClientError};

use buttplug_core::errors::ButtplugError;
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketClientTransport;
use tokio::io::{self, AsyncBufReadExt, BufReader};

async fn wait_for_input() {
  BufReader::new(io::stdin())
    .lines()
    .next_line()
    .await
    .unwrap();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // After you've created a connector, the connection looks the same no
  // matter what, though the errors thrown may be different.
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://127.0.0.1:12345",
  ));

  // Now we connect. If anything goes wrong here, we'll get an Err with either
  //
  // - A ButtplugClientConnectionError if there's a problem with
  //   the Connector, like the network address being wrong, server not
  //   being up, etc.
  // - A ButtplugHandshakeError if there is a client/server version
  //   mismatch.
  let client = ButtplugClient::new("Example Client");
  if let Err(e) = client.connect(connector).await {
    match e {
      ButtplugClientError::ButtplugConnectorError(error) => {
        // If our connection failed, because the server wasn't turned on,
        // SSL/TLS wasn't turned off, etc, we'll just print and exit
        // here.
        println!("Can't connect, exiting! Message: {}", error);
        wait_for_input().await;
        return Ok(());
      }
      ButtplugClientError::ButtplugError(error) => match error {
        ButtplugError::ButtplugHandshakeError(error) => {
          // This means our client is newer than our server, and we need to
          // upgrade the server we're connecting to.
          println!("Handshake issue, exiting! Message: {}", error);
          wait_for_input().await;
          return Ok(());
        }
        error => {
          println!("Unexpected error type! {}", error);
          wait_for_input().await;
          return Ok(());
        }
      },
      _ => {
        // None of the other errors are valid in this instance.
      }
    }
  };

  // We're connected, yay!
  println!("Connected! Check Server for Client Name.");

  wait_for_input().await;

  // And now we disconnect as usual
  client.disconnect().await?;

  Ok(())
}
