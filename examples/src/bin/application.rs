// Buttplug Rust - Complete Application Example
//
// This is a complete, working example that demonstrates the full workflow
// of a Buttplug application. If you're new to Buttplug, start here!
//
// Prerequisites:
// 1. Install Intiface Central: https://intiface.com/central
// 2. Start the server in Intiface Central (click "Start Server")
// 3. Run: cargo run --bin application

use buttplug_client::{
  ButtplugClient, ButtplugClientDevice, ButtplugClientError, ButtplugClientEvent, connector::ButtplugRemoteClientConnector, device::ClientDeviceOutputCommand, serializer::ButtplugClientJSONSerializer
};
use buttplug_core::message::{InputType, OutputType};
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketClientTransport;
use futures::StreamExt;
use tokio::io::{self, AsyncBufReadExt, BufReader};

async fn read_line() -> String {
  BufReader::new(io::stdin())
    .lines()
    .next_line()
    .await
    .unwrap()
    .unwrap_or_default()
}

async fn wait_for_input() {
  let _ = read_line().await;
}

fn print_device_capabilities(device: &ButtplugClientDevice) {
  println!("  {}", device.name());

  // Check output capabilities (things we can make the device do)
  let mut outputs = Vec::new();
  if device.output_available(OutputType::Vibrate) {
    outputs.push("Vibrate");
  }
  /*
  if !device.rotate_features().is_empty() {
    outputs.push("Rotate");
  }
  if !device.oscillate_features().is_empty() {
    outputs.push("Oscillate");
  }
  if !device.position_features().is_empty() {
    outputs.push("Position");
  }
  */

  if !outputs.is_empty() {
    println!("    Outputs: {}", outputs.join(", "));
  }

  // Check input capabilities (sensors we can read)
  let mut inputs = Vec::new();
  if device.input_available(buttplug_core::message::InputType::Battery) {
    inputs.push("Battery");
  }
  if device.input_available(buttplug_core::message::InputType::Rssi) {
    inputs.push("RSSI");
  }

  if !inputs.is_empty() {
    println!("    Inputs: {}", inputs.join(", "));
  }

  println!();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  println!("===========================================");
  println!("  Buttplug Rust Application Example");
  println!("===========================================\n");

  // Step 1: Create a client
  // The client name identifies your application to the server.
  let client = ButtplugClient::new("My Buttplug Application");

  // Step 2: Set up event handlers
  // Get the event stream BEFORE connecting to avoid missing events.
  let mut events = client.event_stream();

  tokio::spawn(async move {
    while let Some(event) = events.next().await {
      match event {
        ButtplugClientEvent::DeviceAdded(device) => {
          println!("[+] Device connected: {}", device.name());
        }
        ButtplugClientEvent::DeviceRemoved(info) => {
          println!("[-] Device disconnected: {}", info.name());
        }
        ButtplugClientEvent::ServerDisconnect => {
          println!("[!] Server connection lost!");
        }
        ButtplugClientEvent::Error(err) => {
          println!("[!] Error: {}", err);
        }
        _ => {}
      }
    }
  });

  // Step 3: Connect to the server
  println!("Connecting to Intiface Central...");
  let connector = ButtplugRemoteClientConnector::<
    ButtplugWebsocketClientTransport,
    ButtplugClientJSONSerializer,
  >::new(ButtplugWebsocketClientTransport::new_insecure_connector(
    "ws://127.0.0.1:12345",
  ));

  if let Err(e) = client.connect(connector).await {
    match e {
      ButtplugClientError::ButtplugConnectorError(error) => {
        println!("ERROR: Could not connect to Intiface Central!");
        println!("Make sure Intiface Central is running and the server is started.");
        println!("Default address: ws://127.0.0.1:12345");
        println!("Error: {}", error);
        return Ok(());
      }
      _ => return Err(e.into()),
    }
  }
  println!("Connected!\n");

  // Step 4: Scan for devices
  println!("Scanning for devices...");
  println!("Turn on your Bluetooth/USB devices now.\n");
  client.start_scanning().await?;

  // Wait for devices (in a real app, you might use a UI or timeout)
  println!("Press Enter when your devices are connected...");
  wait_for_input().await;
  client.stop_scanning().await?;

  // Step 5: Check what devices we found
  let devices: Vec<ButtplugClientDevice> =
    client.devices().into_values().collect();

  if devices.is_empty() {
    println!("No devices found. Make sure your device is:");
    println!("  - Turned on");
    println!("  - In pairing/discoverable mode");
    println!("  - Supported by Buttplug (check https://iostindex.com)");
    client.disconnect().await?;
    return Ok(());
  }

  println!("\nFound {} device(s):\n", devices.len());

  // Step 6: Display device capabilities
  for device in &devices {
    print_device_capabilities(device);
  }

  // Step 7: Interactive device control
  println!("=== Interactive Control ===");
  println!("Commands:");
  println!("  v <0-100>  - Vibrate all devices at percentage");
  println!("  s          - Stop all devices");
  println!("  b          - Read battery levels");
  println!("  q          - Quit\n");

  loop {
    print!("> ");
    // Flush stdout to ensure prompt is visible
    use std::io::Write;
    std::io::stdout().flush().ok();

    let input = read_line().await.trim().to_lowercase();

    if input.is_empty() {
      continue;
    }

    if input.starts_with("v ") {
      // Vibrate command
      if let Ok(percent) = input[2..].parse::<u32>() {
        if percent <= 100 {
          let intensity = percent as f64 / 100.0;
          for device in &devices {
            if !device.output_available(OutputType::Vibrate) {
              match device.run_output(&ClientDeviceOutputCommand::Vibrate(intensity.into())).await {
                Ok(_) => println!("  {}: vibrating at {}%", device.name(), percent),
                Err(e) => println!("  {}: error - {}", device.name(), e),
              }
            }
          }
        } else {
          println!("  Usage: v <0-100>");
        }
      } else {
        println!("  Usage: v <0-100>");
      }
    } else if input == "s" {
      // Stop all devices
      client.stop_all_devices().await?;
      println!("  All devices stopped.");
    } else if input == "b" {
      // Read battery levels
      for device in &devices {
        if device.input_available(InputType::Battery) {
          match device.battery().await {
            Ok(battery) => println!("  {}: {}% battery", device.name(), battery),
            Err(e) => println!("  {}: could not read battery - {}", device.name(), e),
          }
        } else {
          println!("  {}: no battery sensor", device.name());
        }
      }
    } else if input == "q" {
      break;
    } else {
      println!("  Unknown command. Use v, s, b, or q.");
    }
  }

  // Step 8: Clean up
  println!("\nStopping devices and disconnecting...");
  client.stop_all_devices().await?;
  client.disconnect().await?;
  println!("Goodbye!");

  Ok(())
}
