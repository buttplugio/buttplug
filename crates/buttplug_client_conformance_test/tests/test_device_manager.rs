// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client::ButtplugClient;
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_client_conformance_test::server::build_conformance_server;
use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_conformance_devices_appear() {
  // Build conformance server
  let (server, _handles) = build_conformance_server(0)
    .expect("Failed to build conformance server");

  // Create in-process client connector wrapping the server
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();

  // Create and connect client
  let client = ButtplugClient::new("Test Client");
  client
    .connect(connector)
    .await
    .expect("Failed to connect");

  // Subscribe to client's device event stream before scanning
  let mut event_stream = client.event_stream();

  // Call start_scanning
  client
    .start_scanning()
    .await
    .expect("Failed to start scanning");

  // Wait for device events with 5-second timeout
  let mut device_count = 0;

  let result = timeout(Duration::from_secs(5), async {
    while let Some(event) = event_stream.next().await {
      match event {
        buttplug_client::ButtplugClientEvent::DeviceAdded(_device) => {
          device_count += 1;
          if device_count >= 3 {
            return true;
          }
        }
        buttplug_client::ButtplugClientEvent::ScanningFinished => {
          // Scanning finished
          if device_count >= 3 {
            return true;
          }
        }
        _ => {}
      }
    }
    false
  })
  .await;

  // Verify we got the result within timeout
  assert!(
    result.expect("Timeout waiting for devices"),
    "Did not receive enough device events"
  );

  // Assert we have 3 devices
  assert_eq!(
    client.devices().len(),
    3,
    "Expected 3 devices, got {}",
    client.devices().len()
  );
}

#[tokio::test]
async fn test_conformance_device_count_and_handle_access() {
  // Build conformance server and get device handles
  let (server, handles) = build_conformance_server(0)
    .expect("Failed to build conformance server");

  // Verify we got the correct number of handles
  assert_eq!(
    handles.len(),
    3,
    "Expected 3 device handles, got {}",
    handles.len()
  );

  // Create in-process client connector and connect
  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();

  let client = ButtplugClient::new("Test Client");
  client
    .connect(connector)
    .await
    .expect("Failed to connect");

  // Start scanning
  let mut event_stream = client.event_stream();
  client
    .start_scanning()
    .await
    .expect("Failed to start scanning");

  // Wait for devices to appear (at least 3)
  let mut device_count = 0;
  let timeout_result = timeout(Duration::from_secs(5), async {
    while let Some(event) = event_stream.next().await {
      if let buttplug_client::ButtplugClientEvent::DeviceAdded(_device) = event {
        device_count += 1;
        if device_count >= 3 {
          return true;
        }
      }
    }
    false
  })
  .await;

  assert!(
    timeout_result.expect("Timeout waiting for devices"),
    "Did not receive 3 device added events"
  );

  // Verify device handles have the correct structure
  for handle in &handles {
    // Should be able to get the write_log (for capturing commands)
    let _ = handle.write_log.lock().await;
  }
}
