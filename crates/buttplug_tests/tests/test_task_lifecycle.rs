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
    StopCmdV4,
  },
  util::task::registry,
};
use buttplug_server::{device::hardware::HardwareCommand, message::ButtplugClientMessageVariant};
use futures::{StreamExt, pin_mut};
use std::time::Duration;
use util::TestDeviceChannelHost;
use util::stalling_device_communication_manager::StallingDeviceCommunicationManagerBuilder;
use util::{
  test_server_with_comm_manager,
  test_server_with_device,
  test_server_with_device_and_message_gap,
};

/// The Aneros "Massage Demo" stop write for feature 0: `[0xF1, 0x00]`. A vibrate
/// on the same feature writes `[0xF1, 0x40]`; the stop resets it to zero. See
/// `device_test_case/test_aneros_protocol.yaml` for the full sequence.
const ANEROS_STOP_WRITE_FEATURE_0: [u8; 2] = [0xF1, 0x00];

/// Non-blockingly drain every hardware write the test device has recorded so
/// far and return whether any of them is the feature-0 stop write. We use
/// `try_recv` so the check reflects exactly what was on the wire *at the moment
/// stop/shutdown resolved* — a write still sitting in the device io task's batch
/// window has not reached the host channel yet and will not be counted.
fn recorded_a_stop_write(host: &mut TestDeviceChannelHost) -> bool {
  let mut saw_stop = false;
  while let Ok(command) = host.receiver.try_recv() {
    if let HardwareCommand::Write(write) = command
      && write.data().as_slice() == ANEROS_STOP_WRITE_FEATURE_0
    {
      saw_stop = true;
    }
  }
  saw_stop
}

/// Bring a "Massage Demo" device online under `server`, returning its device
/// index. Mirrors the handshake/scan/connect dance the other lifecycle tests do.
async fn bring_device_online(server: &buttplug_server::ButtplugServer, client_name: &str) -> u32 {
  let recv = server.server_version_event_stream();
  pin_mut!(recv);
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        client_name,
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
  .expect("timed out waiting for device to connect")
}

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

  // Device is up. Scope all subsequent leak checks to THIS server's own
  // device-manager subtree: the registry is process-global, so other tests
  // running in parallel must not pollute these assertions. We ask the manager
  // for its own scope path directly rather than guessing it from the global
  // registry snapshot — guessing is racy, because a concurrent test's
  // `device-manager-N` tasks are also "new" relative to our baseline and could
  // be picked instead of ours.
  let scope_prefix: String = server.device_manager().scope_path().to_owned();

  // Sanity: the registry must now show this server spawned per-device tasks
  // under its own subtree.
  let new_tasks: Vec<String> = registry()
    .snapshot()
    .into_iter()
    .map(|t| t.path)
    .filter(|p| !baseline.contains(p) && p.starts_with(&format!("{scope_prefix}/")))
    .collect();
  assert!(
    !new_tasks.is_empty(),
    "expected scope-spawned tasks under {scope_prefix} after a device connected"
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

/// Shutdown-under-load smoke test: with a device connected, scanning still
/// active, and the device in a non-zero output state, `shutdown()` must drive
/// its cleanup (stop_scanning / stop_devices / per-device disconnect) through
/// the live event loop, drain every task under its scope, and return Ok within
/// a bounded time — it must neither hang nor strand tasks.
///
/// SCOPE / what this does NOT verify: this is a smoke test, not a regression
/// test for shutdown *ordering*. It does not prove that cleanup runs *before*
/// scope cancellation — reverting the cleanup-before-cancel ordering leaves this
/// test green, because the contract it checks (shutdown completes and its
/// subtree drains) holds under both orderings with this harness. The
/// cleanup-before-cancel ordering is the correct production behavior, but a
/// test that goes RED on that specific regression is not achievable here (see
/// NOTE). This test guards against the coarser failure mode: a shutdown that
/// hangs or leaks tasks when invoked under realistic load.
///
/// NOTE on why ordering can't be observed here: the stronger "observe the
/// device's actual stop write" assertion is infeasible with this harness. The
/// test hardware sets a 1ms message_gap (see TestHardwareConnector::specialize),
/// so the device io task batches commands; during shutdown the per-device
/// `disconnect()` fires a `Disconnected` hardware event that tears the io task
/// down inside that 1ms batch window, dropping the pending (batched) stop write
/// regardless of cancel ordering. That teardown race is independent of any
/// ordering bug, so a write-observation assertion is inherently flaky here. An
/// instrumented-ordering variant was also attempted and found inherently flaky
/// with this harness, so it is deliberately not pursued.
#[tokio::test]
async fn test_shutdown_under_load_drains_subtree() {
  // Hold the channel so the device stays connected through shutdown.
  let (server, _channel) = test_server_with_device("Massage Demo");

  let recv = server.server_version_event_stream();
  pin_mut!(recv);

  server
    .parse_message(ButtplugClientMessageVariant::V4(
      RequestServerInfoV4::new(
        "Shutdown Under Load Test",
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
  // global registry. We ask the manager directly rather than guessing from the
  // global registry snapshot, which would race with concurrent tests' own
  // `device-manager-N` roots.
  let scope_prefix: String = server.device_manager().scope_path().to_owned();

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

/// Regression test for the cancellable-bringup fix (fix 2): `shutdown()` must
/// not hang when a device bringup is stalled in `connect()`.
///
/// The stalling comm manager emits one `DeviceFound` on scan; the device-manager
/// event loop spawns a bringup task that awaits `connect()`, which never
/// resolves. `shutdown()` cancels the device-manager scope and then
/// `wait_empty_under`s its subtree. The bringup task only deregisters once it
/// observes cancellation via the `biased` select on its token — without that
/// select it would await `connect()` forever and `shutdown()` would never
/// resolve.
///
/// RED evidence: replacing the bringup's `move |token|` select with the
/// non-cancellable `move |_token|` form makes this test time out at the 10s
/// bound and fail. With the fix in place it resolves promptly.
#[tokio::test]
async fn test_shutdown_resolves_with_stalled_bringup() {
  let server = test_server_with_comm_manager(StallingDeviceCommunicationManagerBuilder::default());

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

  // Kick off scanning so a device is found and a bringup task begins — and then
  // stalls in connect().
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StartScanningV0::default().into(),
    ))
    .await
    .expect("start scanning should succeed");

  // Give the bringup task time to spawn and enter (and block in) connect().
  tokio::time::sleep(Duration::from_millis(200)).await;

  // shutdown() must cancel the stalled bringup and return within the bound.
  // Without the biased select on the bringup token this hangs forever.
  tokio::time::timeout(Duration::from_secs(10), server.shutdown())
    .await
    .expect("shutdown hung with a stalled device bringup — bringup is not honoring its cancellation token")
    .expect("server shutdown errored");
}

/// Test A — the stop-write-acknowledgement contract: a `StopCmd` must not
/// resolve until the resulting stop write has actually reached the hardware,
/// even when the device io task is batching commands over a long message gap.
///
/// This retires the limitation documented on `test_shutdown_under_load_drains_subtree`
/// (the 1ms harness gap made write observation flaky). Here we deliberately give
/// the device a 500ms message gap so the batching window is large and the race
/// is deterministic: an active vibrate write lands, the stop write is queued
/// into that 500ms batch, and we assert the stop write is on the wire *by the
/// time the stop message resolves*.
///
/// RED (pre-fix, `git stash` Tasks 1-2): `handle_hardware_commands` fire-and-forgets
/// the stop write into the io channel and `parse_message` for the StopCmd resolves
/// immediately, while the write sits unflushed in the 500ms batch. `try_recv`
/// finds no stop write and this assertion fails deterministically.
///
/// GREEN (with the ack-on-write fix): the stop path requests a write
/// acknowledgement; the io task urgent-flushes the batch and fires the ack, so
/// StopCmd only resolves after the write is on the wire.
#[tokio::test]
async fn test_stop_resolves_only_after_stop_write_reaches_hardware() {
  // 500ms gap: large enough that, pre-fix, the stop write provably has not been
  // flushed by the time StopCmd resolves (which is ~immediate).
  let (server, mut device) =
    test_server_with_device_and_message_gap("Massage Demo", Duration::from_millis(500));

  let device_index = bring_device_online(&server, "Stop Write Ack Test").await;

  // Put the device into an actively-running state so the stop has real work to
  // flush. This vibrate write itself enters the batch window.
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

  // Stop. With ack-on-write, parse_message for StopCmd resolves only after the
  // stop write has been flushed to hardware.
  server
    .parse_message(ButtplugClientMessageVariant::V4(
      StopCmdV4::new(Some(device_index), None, true, true).into(),
    ))
    .await
    .expect("stop command should succeed");

  // The instant stop resolves, the stop write must already be on the wire. No
  // sleep here: that is the whole point — we are asserting ordering, not
  // eventual delivery.
  assert!(
    recorded_a_stop_write(&mut device),
    "StopCmd resolved but the stop write was not yet recorded on the hardware channel — \
     it is still sitting in the device io task's batch window"
  );
}

/// Test B — the original incident: with a batched device in an active output
/// state, `server.shutdown()` must drive the per-device stop write all the way
/// to hardware before it resolves. This is the assertion the task-scope work
/// could not make with the 1ms harness gap (the disconnect tore the io task down
/// inside the batch window, dropping the pending stop write). With ack-on-write,
/// stop_devices waits for the write before shutdown proceeds to disconnect.
///
/// RED (pre-fix): shutdown's stop_devices fire-and-forgets the stop write, then
/// disconnect tears down the io task mid-batch and the write is dropped — no stop
/// write is ever recorded.
///
/// GREEN (with the fix): the stop write is acknowledged before disconnect, so it
/// is recorded before shutdown resolves.
#[tokio::test]
async fn test_shutdown_writes_stop_before_resolving() {
  let (server, mut device) =
    test_server_with_device_and_message_gap("Massage Demo", Duration::from_millis(500));

  let device_index = bring_device_online(&server, "Shutdown Stop Write Test").await;

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

  tokio::time::timeout(Duration::from_secs(10), server.shutdown())
    .await
    .expect("shutdown did not resolve in time")
    .expect("server shutdown errored");

  // By the time shutdown resolved, the stop write must have reached hardware.
  assert!(
    recorded_a_stop_write(&mut device),
    "shutdown() resolved but the device's stop write was never recorded — \
     the pending stop write was dropped when the io task was torn down mid-batch"
  );
}
