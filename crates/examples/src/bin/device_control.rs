use std::collections::HashMap;

use buttplug_client::{connector::ButtplugRemoteClientConnector, serializer::ButtplugClientJSONSerializer, ButtplugClient, ButtplugClientError};

use buttplug_core::message::OutputType;
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketClientTransport;
use strum::IntoEnumIterator;
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
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://127.0.0.1:12345",
  ));

  let client = ButtplugClient::new("Example Client");
  client.connect(connector).await?;

  println!("Connected!");

  // You usually shouldn't run Start/Stop scanning back-to-back like
  // this, but with TestDevice we know our device will be found when we
  // call StartScanning, so we can get away with it.
  client.start_scanning().await?;
  client.stop_scanning().await?;
  println!("Client currently knows about these devices:");
  for device in client.devices() {
    println!("- {}", device.name());
  }
  wait_for_input().await;

  for device in client.devices() {
    println!("{} supports these outputs:", device.name());
    for output_type in OutputType::iter() {
      for (_, feature) in device.device_features() { 
        for (output, _) in feature.feature().output().as_ref().unwrap_or(&HashMap::new()) {
          if output_type == *output {
            println!("- {}", output);
          }
        }
      }
    }
  }

  println!("Sending commands");

  // Now that we know the message types for our connected device, we
  // can send a message over! Seeing as we want to stick with the
  // modern generic messages, we'll go with VibrateCmd.
  //
  // There's a couple of ways to send this message.
  let test_client_device = &client.devices()[0];

  // We can use the convenience functions on ButtplugClientDevice to
  // send the message. This version sets all of the motors on a
  // vibrating device to the same speed.
  test_client_device
    .vibrate(10)
    .await?;

  // If we wanted to just set one motor on and the other off, we could
  // try this version that uses an array. It'll throw an exception if
  // the array isn't the same size as the number of motors available as
  // denoted by FeatureCount, though.
  //
  // You can get the vibrator count using the following code, though we
  // know it's 2 so we don't really have to use it.
  let vibrator_count = test_client_device
    .vibrate_features()
    .len();

  println!(
    "{} has {} vibrators.",
    test_client_device.name(),
    vibrator_count,
  );

  // Just set all of the vibrators to full speed.
  if vibrator_count > 0 {
    test_client_device
      .vibrate(10)
      .await?;
  } else {
    println!("Device does not have > 1 vibrators, not running multiple vibrator test.");
  }

  wait_for_input().await;
  println!("Disconnecting");
  // And now we disconnect as usual.
  client.disconnect().await?;
  println!("Trying error");
  // If we try to send a command to a device after the client has
  // disconnected, we'll get an exception thrown.
  let vibrate_result = test_client_device
    .vibrate(30)
    .await;
  if let Err(ButtplugClientError::ButtplugConnectorError(error)) = vibrate_result {
    println!("Tried to send after disconnection! Error: ");
    println!("{}", error);
  }
  wait_for_input().await;

  Ok(())
}
