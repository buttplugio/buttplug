use buttplug::{
  client::{device::ScalarValueCommand, ButtplugClient, ButtplugClientError},
  core::{
    connector::{
      new_json_ws_client_connector,
    },
    message::{ClientGenericDeviceMessageAttributes},
  },
};
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
  let connector = new_json_ws_client_connector("ws://localhost:12345");

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
    fn print_attrs(attrs: &Vec<ClientGenericDeviceMessageAttributes>) {
      for attr in attrs {
        println!(
          "{}: {} - Steps: {}",
          attr.actuator_type(),
          attr.feature_descriptor(),
          attr.step_count()
        );
      }
    }
    println!("{} supports these actions:", device.name());
    if let Some(attrs) = device.message_attributes().scalar_cmd() {
      print_attrs(attrs);
    }
    print_attrs(&device.rotate_attributes());
    print_attrs(&device.linear_attributes());
    println!("Battery: {}", device.has_battery_level());
    println!("RSSI: {}", device.has_rssi_level());
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
    .vibrate(&ScalarValueCommand::ScalarValue(1.0))
    .await?;

  // If we wanted to just set one motor on and the other off, we could
  // try this version that uses an array. It'll throw an exception if
  // the array isn't the same size as the number of motors available as
  // denoted by FeatureCount, though.
  //
  // You can get the vibrator count using the following code, though we
  // know it's 2 so we don't really have to use it.
  let vibrator_count = test_client_device
    .vibrate_attributes()
    .len();

  println!(
    "{} has {} vibrators.",
    test_client_device.name(),
    vibrator_count,
  );

  // Just set all of the vibrators to full speed.
  if vibrator_count > 1 {
    test_client_device
      .vibrate(&ScalarValueCommand::ScalarValueVec(vec![1.0, 0.0]))
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
    .vibrate(&ScalarValueCommand::ScalarValue(1.0))
    .await;
  if let Err(ButtplugClientError::ButtplugConnectorError(error)) = vibrate_result {
    println!("Tried to send after disconnection! Error: ");
    println!("{}", error);
  }
  wait_for_input().await;

  Ok(())
}
