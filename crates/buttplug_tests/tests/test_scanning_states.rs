// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Tests for the server scanning state machine.
//!
//! The server device manager has a ScanningState state machine:
//!   Idle → BringupInProgress → Active → ActiveStopRequested → Idle
//!
//! These tests verify state transitions work correctly, especially for comm
//! managers like btleplug that never emit ScanningFinished (long-running BLE scans).
//! Regression tests for intiface/intiface-central#246.

mod util;

use buttplug_client::{ButtplugClient, ButtplugClientEvent};
use buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder;
use buttplug_server::{ButtplugServerBuilder, device::ServerDeviceManagerBuilder};
use futures::StreamExt;
use std::sync::atomic::Ordering;
use std::time::Duration;
use util::{
  create_test_dcm,
  long_running_scan_comm_manager::{
    LongRunningScanCommunicationManagerBuilder, LongRunningScanState,
  },
  test_device_manager::TestDeviceIdentifier,
};

/// Helper: create a client wired to a server with the long-running scan comm manager.
async fn setup_long_running_scan_client(
  state: &LongRunningScanState,
  devices: &[&str],
) -> ButtplugClient {
  let mut builder = LongRunningScanCommunicationManagerBuilder::new(state.clone());
  for name in devices {
    // We don't need the host channel for scanning tests — just need devices to exist
    let _ = builder.add_test_device(&TestDeviceIdentifier::new(name, None));
  }

  let mut dm_builder = ServerDeviceManagerBuilder::new(create_test_dcm());
  dm_builder.comm_manager(builder);

  let server = ButtplugServerBuilder::new(dm_builder.finish().unwrap())
    .finish()
    .unwrap();

  let connector = ButtplugInProcessClientConnectorBuilder::default()
    .server(server)
    .finish();

  let client = ButtplugClient::new("Scanning State Test Client");
  client
    .connect(connector)
    .await
    .expect("Test, assuming infallible.");
  client
}

/// Helper: create a client wired to a server with the delay (ScanningFinished-emitting) comm manager.
async fn setup_delay_scan_client() -> ButtplugClient {
  util::test_client_with_delayed_device_manager().await
}

// =============================================================================
// Regression test: rescan after stop with a btleplug-like comm manager
// This is the core test for intiface/intiface-central#246
// =============================================================================

/// The scanning state machine must return to Idle after stop_scanning, even when
/// the comm manager never sends ScanningFinished. Without the fix, the state gets
/// stuck in ActiveStopRequested and silently rejects all subsequent start_scanning.
#[tokio::test]
async fn test_rescan_works_without_scanning_finished() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &["Massage Demo"]).await;

  // Scan #1
  client.start_scanning().await.expect("Start scanning #1");
  // Give the async scan task time to run
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(state.start_count.load(Ordering::Relaxed), 1);

  client.stop_scanning().await.expect("Stop scanning #1");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(state.stop_count.load(Ordering::Relaxed), 1);
  assert!(!state.is_scanning.load(Ordering::Relaxed));

  // Scan #2 — this is the regression: previously stuck in ActiveStopRequested
  client.start_scanning().await.expect("Start scanning #2");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(
    state.start_count.load(Ordering::Relaxed),
    2,
    "start_scanning must be called twice — second scan was silently rejected"
  );

  client.stop_scanning().await.expect("Stop scanning #2");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(state.stop_count.load(Ordering::Relaxed), 2);

  client.disconnect().await.expect("Disconnect");
}

/// Multiple stop/start cycles should all work with a btleplug-like comm manager.
#[tokio::test]
async fn test_multiple_rescan_cycles_without_scanning_finished() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &["Massage Demo"]).await;

  for i in 1..=5u32 {
    client
      .start_scanning()
      .await
      .unwrap_or_else(|_| panic!("Start scanning #{i}"));
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert_eq!(state.start_count.load(Ordering::Relaxed), i);

    client
      .stop_scanning()
      .await
      .unwrap_or_else(|_| panic!("Stop scanning #{i}"));
    tokio::time::sleep(Duration::from_millis(25)).await;
    assert_eq!(state.stop_count.load(Ordering::Relaxed), i);
  }

  client.disconnect().await.expect("Disconnect");
}

// =============================================================================
// Baseline: rescan with a comm manager that DOES send ScanningFinished
// =============================================================================

/// Rescan should also work with a comm manager that sends ScanningFinished (the
/// existing behavior for timed-retry managers like HID/serial).
#[tokio::test]
async fn test_rescan_works_with_scanning_finished() {
  let client = setup_delay_scan_client().await;

  client.start_scanning().await.expect("Start scanning #1");
  tokio::time::sleep(Duration::from_millis(50)).await;

  client.stop_scanning().await.expect("Stop scanning #1");
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Should not be rejected
  client.start_scanning().await.expect("Start scanning #2");
  tokio::time::sleep(Duration::from_millis(50)).await;

  client.stop_scanning().await.expect("Stop scanning #2");

  client.disconnect().await.expect("Disconnect");
}

// =============================================================================
// Device discovery across scan cycles
// =============================================================================

/// Devices found during scan #1 should trigger DeviceAdded events. After stop+start,
/// the server should attempt to discover devices again (even if duplicates are filtered).
#[tokio::test]
async fn test_device_found_events_sent_on_rescan() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &["Massage Demo"]).await;
  let mut events = client.event_stream();

  // Scan #1
  client.start_scanning().await.expect("Start scanning #1");

  // Wait for DeviceAdded (Massage Demo should be recognized as "Aneros Vivi")
  // The first event may be DeviceListReceived, so skip non-DeviceAdded events.
  let mut found_device = false;
  let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
  while tokio::time::Instant::now() < deadline {
    match tokio::time::timeout_at(deadline, events.next()).await {
      Ok(Some(ButtplugClientEvent::DeviceAdded(device))) => {
        assert_eq!(device.name(), "Aneros Vivi");
        found_device = true;
        break;
      }
      Ok(Some(_)) => continue, // Skip DeviceListReceived etc.
      Ok(None) => break,
      Err(_) => break,
    }
  }
  assert!(found_device, "DeviceAdded event never received");

  client.stop_scanning().await.expect("Stop scanning #1");
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Scan #2 — the comm manager will emit DeviceFound again, but the server should
  // filter the duplicate address. The important thing is that start_scanning was called.
  client.start_scanning().await.expect("Start scanning #2");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(
    state.start_count.load(Ordering::Relaxed),
    2,
    "Comm manager must receive second start_scanning call"
  );

  client.disconnect().await.expect("Disconnect");
}

// =============================================================================
// Edge cases
// =============================================================================

/// Calling stop_scanning when not scanning should be harmless.
#[tokio::test]
async fn test_stop_scanning_when_idle() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &[]).await;

  // Stop without ever starting — should not error
  client.stop_scanning().await.expect("Stop when idle");

  // Should still be able to start scanning afterward
  client.start_scanning().await.expect("Start scanning");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(state.start_count.load(Ordering::Relaxed), 1);

  client.disconnect().await.expect("Disconnect");
}

/// Calling start_scanning while already scanning should be handled gracefully.
#[tokio::test]
async fn test_start_scanning_while_already_scanning() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &["Massage Demo"]).await;

  client.start_scanning().await.expect("Start scanning #1");
  tokio::time::sleep(Duration::from_millis(50)).await;

  // Second start while still scanning — should be silently ignored by the server
  // (the comm manager should NOT receive a second start_scanning)
  client.start_scanning().await.expect("Start scanning #2");
  tokio::time::sleep(Duration::from_millis(50)).await;
  assert_eq!(
    state.start_count.load(Ordering::Relaxed),
    1,
    "Duplicate start_scanning while active should be ignored"
  );

  client.stop_scanning().await.expect("Stop scanning");
  client.disconnect().await.expect("Disconnect");
}

/// Rapid stop+start should work without race conditions.
#[tokio::test]
async fn test_rapid_stop_start_cycle() {
  let state = LongRunningScanState::default();
  let client = setup_long_running_scan_client(&state, &["Massage Demo"]).await;

  client.start_scanning().await.expect("Start");
  tokio::time::sleep(Duration::from_millis(25)).await;

  // Rapid stop+start without waiting
  client.stop_scanning().await.expect("Stop");
  client.start_scanning().await.expect("Re-start");
  tokio::time::sleep(Duration::from_millis(50)).await;

  assert_eq!(state.start_count.load(Ordering::Relaxed), 2);
  assert_eq!(state.stop_count.load(Ordering::Relaxed), 1);

  client.disconnect().await.expect("Disconnect");
}
