// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod util;

use buttplug_core::{
  message::{
    BUTTPLUG_CURRENT_API_MAJOR_VERSION,
    BUTTPLUG_CURRENT_API_MINOR_VERSION,
    ButtplugServerMessageV4,
    OutputCmdV4,
    OutputCommand,
    OutputValue,
    RequestServerInfoV4,
    StartScanningV0,
  },
  util::task::registry,
};
use buttplug_server::message::ButtplugClientMessageVariant;
use futures::{StreamExt, pin_mut};
use std::time::Duration;
use util::test_server_with_device;

/// Bring a real (test-harness) device online and confirm that, once the server
/// is shut down and dropped, no tasks remain registered under the scope tree.
///
/// This exercises the *device task* lifecycle specifically: a connected device
/// spawns an `io` task whose only exit path is its `token.cancelled()` select
/// arm. If scope cancellation were removed, that task would leak and this test
/// would fail with a non-empty leaked-task list. (A server with no device is
/// insufficient — the device-manager event loop also exits on channel drop, so
/// it cannot prove cancellation actually fires.)
#[tokio::test]
async fn test_server_shutdown_leaves_no_tasks() {
  let baseline: Vec<String> = registry().snapshot().into_iter().map(|t| t.path).collect();

  // Hold the channel so the device stays connected.
  let (server, _channel) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Task Lifecycle Test",
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

  // Wait until the device is actually connected — this is what spawns the
  // per-device `io` task that we want to prove gets cleaned up.
  tokio::time::timeout(Duration::from_secs(5), async {
    while let Some(msg) = recv.next().await {
      if let ButtplugServerMessageV4::DeviceList(list) = msg {
        if !list.devices().is_empty() {
          return;
        }
      }
    }
    panic!("device event stream ended before a device connected");
  })
  .await
  .expect("timed out waiting for device to connect");

  // Device is up: the registry must now show more tasks than baseline, and at
  // least one of them must be a per-device task. We also derive this server's
  // own device-manager scope prefix so subsequent leak checks inspect only this
  // server's subtree — the registry is process-global, so other tests running
  // in parallel must not pollute these assertions.
  let after_connect: Vec<String> = registry().snapshot().into_iter().map(|t| t.path).collect();
  let new_tasks: Vec<&String> = after_connect
    .iter()
    .filter(|p| !baseline.contains(p))
    .collect();
  assert!(
    !new_tasks.is_empty(),
    "expected scope-spawned tasks after a device connected"
  );
  assert!(
    new_tasks.iter().any(|p| p.contains("/device-")),
    "expected a per-device task in the registry, got: {new_tasks:?}"
  );
  let scope_prefix: String = new_tasks
    .iter()
    .find_map(|p| {
      p.split_once('/')
        .filter(|(root, _)| root.starts_with("device-manager"))
        .map(|(root, _)| root.to_owned())
    })
    .expect("expected this server's device-manager scope in the registry");

  // `shutdown()` is contractually responsible for draining every task it
  // spawned: it cancels the scope tree and awaits the registry going empty
  // under its path before returning. We assert emptiness *before* dropping the
  // server, so that ordinary drop-of-channels teardown cannot mask a missing
  // cancellation arm. If any scope-spawned task fails to observe cancellation,
  // it will still be parked on its event stream / receiver here and show up as
  // leaked. We hold `_channel` alive across this check precisely so the device
  // event stream does not close on its own.
  server.shutdown().await.expect("server shutdown errored");

  let leaked = registry().live_count_under(&scope_prefix);
  assert_eq!(
    leaked, 0,
    "shutdown() returned but {leaked} task(s) are still registered under {scope_prefix}"
  );

  // Dropping the server must not resurrect or strand anything either. Give a
  // short grace period and confirm this server's subtree stays empty.
  drop(server);
  drop(_channel);
  tokio::time::timeout(Duration::from_secs(5), async {
    loop {
      if registry().live_count_under(&scope_prefix) == 0 {
        return;
      }
      tokio::time::sleep(Duration::from_millis(50)).await;
    }
  })
  .await
  .unwrap_or_else(|_| {
    let leaked = registry().live_count_under(&scope_prefix);
    panic!("leaked {leaked} task(s) under {scope_prefix} after drop");
  });
}

/// Regression test for shutdown ordering: cleanup MUST run before the task
/// scope is cancelled.
///
/// `shutdown()` must run stop_scanning / stop_devices / per-device disconnect
/// and cancel the device-manager scope only afterwards. The buggy ordering
/// cancelled the scope synchronously first: device io tasks select `biased`
/// with their cancellation token, so queued stop commands were dropped, and the
/// StopScanning issued into the still-running event loop raced its cancellation.
///
/// This test exercises the StopScanning-through-event-loop path the old
/// ordering broke: a device is connected AND scanning is still active when
/// `shutdown()` is called. `shutdown()` must drive cleanup through the live
/// event loop, drain every scope task, and return Ok within a bounded time —
/// i.e. it must not hang or strand tasks.
///
/// NOTE on variant choice: the stronger "observe the device's actual stop
/// write" assertion proved infeasible with this harness. The test hardware sets
/// a 1ms message_gap (see TestHardwareConnector::specialize), so the device io
/// task batches commands; during shutdown the per-device `disconnect()` fires a
/// `Disconnected` hardware event that tears the io task down inside that 1ms
/// batch window, dropping the pending (batched) stop write regardless of cancel
/// ordering. That teardown race is independent of the bug under test, so a
/// write-observation assertion is inherently flaky here. We therefore assert the
/// contract the cleanup-before-cancel ordering must uphold: shutdown completes
/// successfully with cleanup driven through the live event loop.
#[tokio::test]
async fn test_shutdown_runs_cleanup_through_event_loop_before_cancel() {
  // Capture baseline so we can confirm shutdown drains everything it spawned.
  let baseline: Vec<String> = registry().snapshot().into_iter().map(|t| t.path).collect();

  // Hold the channel so the device stays connected through shutdown.
  let (server, _channel) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Shutdown Ordering Test",
        BUTTPLUG_CURRENT_API_MAJOR_VERSION,
        BUTTPLUG_CURRENT_API_MINOR_VERSION,
      )
      .into(),
    ))
    .await
    .expect("server info request should succeed");

  // Start scanning and leave it running: shutdown's stop_scanning must drain
  // through the event loop before the scope is cancelled.
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("start scanning should succeed");

  // Wait for the device to connect so its io / event-forwarding tasks exist
  // under the scope alongside the still-running scanning state.
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

  // Put the device into an actively-running state so there is real cleanup to
  // perform (a non-zero output that StopCmd must reset).
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

  // Identify this server's own device-manager scope prefix so the leak check
  // below is isolated from any other test running in parallel against the
  // global registry.
  let scope_prefix: String = registry()
    .snapshot()
    .into_iter()
    .map(|t| t.path)
    .filter(|p| !baseline.contains(p))
    .find_map(|p| {
      p.split_once('/')
        .filter(|(root, _)| root.starts_with("device-manager"))
        .map(|(root, _)| root.to_owned())
    })
    .expect("expected this server's device-manager scope in the registry");

  // shutdown() must drive cleanup (stop_scanning + stop_devices + disconnect)
  // through the live event loop and only then cancel the scope, returning Ok
  // within a bounded time.
  tokio::time::timeout(Duration::from_secs(10), server.shutdown())
    .await
    .expect("shutdown did not resolve in time — cleanup likely raced against cancellation")
    .expect("server shutdown errored");

  // shutdown() is contractually responsible for draining every task under its
  // own scope. Inspect only this server's subtree to stay parallel-safe.
  let leaked = registry().live_count_under(&scope_prefix);
  assert_eq!(
    leaked, 0,
    "shutdown() returned but {leaked} task(s) are still registered under {scope_prefix}"
  );
}
