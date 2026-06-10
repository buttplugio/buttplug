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
  // least one of them must be a per-device task.
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

  // `shutdown()` is contractually responsible for draining every task it
  // spawned: it cancels the scope tree and awaits the registry going empty
  // under its path before returning. We assert emptiness *before* dropping the
  // server, so that ordinary drop-of-channels teardown cannot mask a missing
  // cancellation arm. If any scope-spawned task fails to observe cancellation,
  // it will still be parked on its event stream / receiver here and show up as
  // leaked. We hold `_channel` alive across this check precisely so the device
  // event stream does not close on its own.
  server.shutdown().await.expect("server shutdown errored");

  let leaked: Vec<String> = registry()
    .snapshot()
    .into_iter()
    .map(|t| t.path)
    .filter(|p| !baseline.contains(p))
    .collect();
  assert!(
    leaked.is_empty(),
    "shutdown() returned but tasks are still registered: {leaked:?}"
  );

  // Dropping the server must not resurrect or strand anything either. Give a
  // short grace period and confirm we remain at (or below) baseline.
  drop(server);
  drop(_channel);
  tokio::time::timeout(Duration::from_secs(5), async {
    loop {
      let now: Vec<String> = registry().snapshot().into_iter().map(|t| t.path).collect();
      if now.iter().filter(|p| !baseline.contains(p)).count() == 0 {
        return;
      }
      tokio::time::sleep(Duration::from_millis(50)).await;
    }
  })
  .await
  .unwrap_or_else(|_| {
    let leaked: Vec<String> = registry()
      .snapshot()
      .into_iter()
      .map(|t| t.path)
      .filter(|p| !baseline.contains(p))
      .collect();
    panic!("leaked tasks after drop: {leaked:?}");
  });
}
