// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Task-lifecycle integration tests.
//!
//! These tests assert the *observable contract* of server shutdown driven by the
//! owner-local TaskGroup: cleanup side effects land (a running device receives a
//! stop write before it is torn down), shutdown resolves in a bounded time even
//! when work is still in flight, and concurrent / repeated callers share a single
//! shutdown result. They deliberately avoid any global task registry: they observe
//! the device's hardware-command channel and the join signal of shutdown itself.

mod util;

use std::time::Duration;

use buttplug_core::message::{
  BUTTPLUG_CURRENT_API_MAJOR_VERSION,
  BUTTPLUG_CURRENT_API_MINOR_VERSION,
  ButtplugServerMessageV4,
  OutputCmdV4,
  OutputCommand,
  OutputValue,
  RequestServerInfoV4,
  StartScanningV0,
};
use buttplug_server::device::hardware::HardwareCommand;
use buttplug_server::message::ButtplugClientMessageVariant;
use buttplug_server_device_config::Endpoint;
use futures::StreamExt;
use futures::pin_mut;
use util::{
  stalling_device_communication_manager::StallingDeviceCommunicationManagerBuilder,
  test_device_manager::TestHardwareEvent,
  test_server_with_comm_manager,
  test_server_with_device,
};

/// Brings up a real (test-hardware) device, puts it into a running state, then
/// shuts the server down. The shutdown sequence must send a Stop through the live
/// event loop *before* cancelling / joining its tasks, so the device's hardware
/// channel must observe a zeroing write. Shutdown must also resolve Ok within a
/// bounded time — if task cancellation raced ahead of cleanup, the stop write
/// would be dropped and shutdown would still return, but the device would keep
/// running; if join were skipped, this test would hang.
#[tokio::test]
async fn test_shutdown_flushes_stop_to_hardware_before_join() {
  let timeout = Duration::from_secs(10);
  let (server, mut device) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Shutdown Stop Flush Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("server info request should succeed");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("start scanning should succeed");

  // Wait for the device to connect so its io task exists under the manager scope.
  let device_index = tokio::time::timeout(Duration::from_secs(5), async {
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessageV4::DeviceList(list) = msg
        && let Some((&idx, _)) = list.devices().iter().next()
      {
        return idx;
      }
    }
    panic!("device event stream ended before a device connected");
  })
  .await
  .expect("timed out waiting for device to connect");

  // Put the device into an actively-running state so Stop has real work to flush.
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      OutputCmdV4::new(
        device_index,
        0,
        OutputCommand::Vibrate(OutputValue::new(50)),
      )
      .into(),
    ))
    .await
    .expect("vibrate command should succeed");

  // Drain the vibrate write off the channel so only the stop write remains.
  let _ = tokio::time::timeout(timeout, device.receiver.recv())
    .await
    .expect("timed out waiting for vibrate write");

  // shutdown() must drive stop through the live event loop before it cancels the
  // task scope, so the zeroing write must land on the device channel.
  let shutdown_result = tokio::time::timeout(timeout, server.shutdown()).await;

  // The stop write is the side effect we care about: assert it landed regardless
  // of whether shutdown has already torn the channel down.
  let stop_write = tokio::time::timeout(timeout, device.receiver.recv()).await;
  assert!(
    matches!(
      stop_write,
      Ok(Some(HardwareCommand::Write(ref w))) if w.endpoint() == Endpoint::Tx
    ),
    "shutdown did not flush a stop write to the device before joining; got {stop_write:?}"
  );

  shutdown_result
    .expect("shutdown did not resolve in time — join likely skipped or deadlocked")
    .expect("server shutdown errored");
}

/// Concurrent shutdown callers must observe the same result from the shared
/// single-flight shutdown future. Both callers must resolve without a duplicate
/// cleanup panic or hang, and cleanup must still reach the hardware.
#[tokio::test]
async fn test_concurrent_shutdown_callers_share_result() {
  let timeout = Duration::from_secs(10);
  let (server, mut device) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Concurrent Shutdown Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("server info request should succeed");

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("start scanning should succeed");

  tokio::time::timeout(Duration::from_secs(5), async {
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessageV4::DeviceList(list) = msg
        && let Some((&idx, _)) = list.devices().iter().next()
      {
        return idx;
      }
    }
    panic!("device event stream ended before a device connected");
  })
  .await
  .expect("timed out waiting for device to connect");

  // Drain any writes the device produced during connect (e.g. keepalive replay
  // or an initial batch) so the channel is quiet before shutdown runs.
  while let Ok(Some(_)) =
    tokio::time::timeout(Duration::from_millis(20), device.receiver.recv()).await
  {}

  // Two concurrent callers against the same server.
  let (first, second) = futures::future::join(server.shutdown(), server.shutdown()).await;

  first.expect("first shutdown caller errored");
  second.expect("second shutdown caller errored");

  // Cleanup must still reach hardware. A single Massage Demo stop can emit
  // multiple zeroing writes, so write count is not a cleanup execution count.
  let stop_write = tokio::time::timeout(timeout, device.receiver.recv()).await;
  assert!(
    matches!(
      stop_write,
      Ok(Some(HardwareCommand::Write(ref w))) if w.endpoint() == Endpoint::Tx
    ),
    "shared shutdown did not reach hardware; got {stop_write:?}"
  );
}

/// A repeated (sequential) shutdown must be a cheap no-op that returns the same
/// outcome as the first call, never panicking on a closed task scope.
#[tokio::test]
async fn test_repeated_shutdown_is_idempotent() {
  let timeout = Duration::from_secs(10);
  let (server, _device) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Repeated Shutdown Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("server info request should succeed");

  tokio::time::timeout(timeout, server.shutdown())
    .await
    .expect("first shutdown did not resolve in time")
    .expect("first shutdown errored");

  // Second call against an already-shutdown server must still resolve Ok quickly
  // — it must not hang on a join or panic on a closed scope.
  tokio::time::timeout(timeout, server.shutdown())
    .await
    .expect("second shutdown did not resolve in time")
    .expect("second shutdown errored");
}

#[tokio::test]
async fn test_shutdown_resolves_with_stalled_bringup() {
  let server = test_server_with_comm_manager(StallingDeviceCommunicationManagerBuilder);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Stalled Bringup Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("server info request should succeed");
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("start scanning should succeed");

  tokio::time::sleep(Duration::from_millis(200)).await;
  tokio::time::timeout(Duration::from_secs(10), server.shutdown())
    .await
    .expect("shutdown hung with a stalled device bringup")
    .expect("server shutdown errored");
}

#[allow(dead_code)]
fn _reference_test_hardware_event(_e: TestHardwareEvent) {
}
