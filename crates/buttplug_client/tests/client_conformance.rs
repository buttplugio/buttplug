// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Integration tests that exercise the Rust ButtplugClient API against the
//! conformance test devices (in-process, no WebSocket). These tests validate
//! that the client correctly translates high-level API calls into the protocol
//! bytes that reach the simulated hardware layer.

use buttplug_client::{
  ButtplugClient, ButtplugClientDevice, ButtplugClientDeviceEvent, ButtplugClientEvent,
  device::{ClientDeviceCommandValue, ClientDeviceOutputCommand},
};
use buttplug_client_conformance_test::{
  device_manager::ConformanceDeviceHandle, server::build_conformance_server,
};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_server::device::hardware::HardwareEvent;
use futures::StreamExt;

/// Build a connected client and return it with the conformance device handles.
/// Scans for devices and waits until all 3 conformance devices appear.
async fn setup_client() -> (ButtplugClient, Vec<ButtplugClientDevice>, Vec<ConformanceDeviceHandle>)
{
  let (server, device_handles) = build_conformance_server(0).expect("conformance server build");

  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();

  let client = ButtplugClient::new("Rust Client Conformance Test");
  client.connect(connector).await.expect("connect");

  let mut events = client.event_stream();
  client.start_scanning().await.expect("start_scanning");

  // Collect DeviceAdded events until we have 3 devices or ScanningFinished
  let mut devices_seen = 0usize;
  loop {
    match tokio::time::timeout(std::time::Duration::from_secs(5), events.next()).await {
      Ok(Some(ButtplugClientEvent::DeviceAdded(_))) => {
        devices_seen += 1;
        if devices_seen >= 3 {
          break;
        }
      }
      Ok(Some(ButtplugClientEvent::ScanningFinished)) => break,
      Ok(Some(_)) => continue,
      Ok(None) => panic!("event stream closed before 3 devices appeared"),
      Err(_) => panic!("timed out waiting for devices"),
    }
  }

  // Collect devices in stable index order
  let mut devices: Vec<ButtplugClientDevice> = client.devices().into_values().collect();
  devices.sort_by_key(|d| d.index());

  assert_eq!(devices.len(), 3, "expected 3 conformance devices");

  (client, devices, device_handles)
}

/// Asserts that the write log for a device has at least one Tx write, and that
/// the most recent write targets the expected feature index byte.
async fn assert_write_log(handle: &ConformanceDeviceHandle, expected_feature_index: u8) {
  let log = handle.write_log.lock().await;
  assert!(!log.is_empty(), "no writes recorded for device");
  let last = log.last().unwrap();
  let data = last.data();
  assert!(
    data.len() >= 5,
    "write data too short: {} bytes",
    data.len()
  );
  assert_eq!(
    data[0], expected_feature_index,
    "expected feature index {} in write, got {}",
    expected_feature_index, data[0]
  );
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_connect_and_enumerate() {
  let (client, devices, _handles) = setup_client().await;

  assert!(client.connected());
  assert_eq!(
    client.server_name().as_deref(),
    Some("Buttplug Conformance Test Server")
  );

  // Confirm the 3 device names match the conformance definitions
  let names: Vec<&str> = devices.iter().map(|d| d.name().as_str()).collect();
  assert!(
    names.iter().any(|n| *n == "Conformance Test Vibrator"),
    "missing Vibrator: {:?}",
    names
  );
  assert!(
    names.iter().any(|n| *n == "Conformance Test Positioner"),
    "missing Positioner: {:?}",
    names
  );
  assert!(
    names.iter().any(|n| *n == "Conformance Test Multi"),
    "missing Multi: {:?}",
    names
  );
}

#[tokio::test]
async fn test_output_commands() {
  let (_client, devices, handles) = setup_client().await;

  // Device 0: Vibrator
  // Feature 0 – Vibrate
  devices[0]
    .device_features()
    .get(&0)
    .expect("feature 0")
    .run_output(&ClientDeviceOutputCommand::Vibrate(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("vibrate feature 0");
  assert_write_log(&handles[0], 0).await;

  // Feature 1 – Vibrate (second motor)
  devices[0]
    .device_features()
    .get(&1)
    .expect("feature 1")
    .run_output(&ClientDeviceOutputCommand::Vibrate(
      ClientDeviceCommandValue::Percent(0.75),
    ))
    .await
    .expect("vibrate feature 1");
  assert_write_log(&handles[0], 1).await;

  // Feature 2 – Rotate
  devices[0]
    .device_features()
    .get(&2)
    .expect("feature 2")
    .run_output(&ClientDeviceOutputCommand::Rotate(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("rotate feature 2");
  assert_write_log(&handles[0], 2).await;

  // Device 1: Positioner
  // Feature 0 – Position
  devices[1]
    .device_features()
    .get(&0)
    .expect("feature 0")
    .run_output(&ClientDeviceOutputCommand::Position(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("position feature 0");
  assert_write_log(&handles[1], 0).await;

  // Feature 2 – Oscillate
  devices[1]
    .device_features()
    .get(&2)
    .expect("feature 2")
    .run_output(&ClientDeviceOutputCommand::Oscillate(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("oscillate feature 2");
  assert_write_log(&handles[1], 2).await;

  // Device 2: Multi
  // Feature 0 – Constrict
  devices[2]
    .device_features()
    .get(&0)
    .expect("feature 0")
    .run_output(&ClientDeviceOutputCommand::Constrict(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("constrict feature 0");
  assert_write_log(&handles[2], 0).await;

  // Feature 1 – Spray
  devices[2]
    .device_features()
    .get(&1)
    .expect("feature 1")
    .run_output(&ClientDeviceOutputCommand::Spray(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("spray feature 1");
  assert_write_log(&handles[2], 1).await;

  // Feature 2 – Temperature
  devices[2]
    .device_features()
    .get(&2)
    .expect("feature 2")
    .run_output(&ClientDeviceOutputCommand::Temperature(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("temperature feature 2");
  assert_write_log(&handles[2], 2).await;

  // Feature 3 – Led
  devices[2]
    .device_features()
    .get(&3)
    .expect("feature 3")
    .run_output(&ClientDeviceOutputCommand::Led(
      ClientDeviceCommandValue::Percent(0.5),
    ))
    .await
    .expect("led feature 3");
  assert_write_log(&handles[2], 3).await;
}

#[tokio::test]
async fn test_stop_commands() {
  let (client, devices, handles) = setup_client().await;

  // Send a vibrate so the device has at least one write
  devices[0]
    .device_features()
    .get(&0)
    .expect("feature 0")
    .run_output(&ClientDeviceOutputCommand::Vibrate(
      ClientDeviceCommandValue::Percent(1.0),
    ))
    .await
    .expect("vibrate before stop");

  let writes_before = handles[0].write_log.lock().await.len();
  assert!(writes_before > 0, "expected at least one write before stop");

  // stop_all_devices sends a StopCmd to the server which zeroes all devices
  client.stop_all_devices().await.expect("stop_all_devices");

  // Each device should have received an additional zero-value write
  tokio::time::sleep(std::time::Duration::from_millis(50)).await;
  let writes_after = handles[0].write_log.lock().await.len();
  assert!(
    writes_after > writes_before,
    "stop_all_devices should have produced additional writes"
  );
}

#[tokio::test]
async fn test_device_removal_event() {
  let (_client, devices, handles) = setup_client().await;

  // Subscribe to device 1's event stream before simulating removal
  let device1 = devices
    .iter()
    .find(|d| d.name() == "Conformance Test Positioner")
    .expect("Positioner device");

  let mut device_events = device1.event_stream();

  // Simulate hardware disconnect for the positioner (index 1 in the conformance device definitions)
  handles[1]
    .event_sender
    .send(HardwareEvent::Disconnected(
      "Conformance Test Positioner".to_string(),
    ))
    .ok();

  // Wait for DeviceRemoved event on the device's own stream
  let event = tokio::time::timeout(
    std::time::Duration::from_secs(5),
    device_events.next(),
  )
  .await
  .expect("timed out waiting for device removal event")
  .expect("device event stream closed");

  assert!(
    matches!(event, ButtplugClientDeviceEvent::DeviceRemoved),
    "expected DeviceRemoved, got {:?}",
    event
  );

  assert!(
    !device1.connected(),
    "device should report disconnected after removal"
  );
}
