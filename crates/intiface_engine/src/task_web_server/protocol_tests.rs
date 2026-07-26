// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Unit tests for DTO conversion, deterministic snapshot ordering, and the
//! bootstrap reconciliation / lag-recovery logic driven against a scripted
//! [`super::ScriptedTaskEventSource`]. These exercise the race/lag contract
//! without going through the process-global registry or an HTTP layer.
//!
//! All ids here are the scripted source's local `u64` ids — the tests never
//! touch `buttplug_core` task types or the global registry.

use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::ScriptedTaskEventSource as Source;
use super::protocol::{
  DtoConvert, SourceOutcome, SourceTaskEntry, StreamItem, TaskEventSource, drive_typed,
  outcome_str, sorted_snapshot,
};

/// Drive the scripted source until a deadline, collecting typed items. The
/// driver observes a cancellation token that is never cancelled here.
async fn collect_items(source: Source, deadline_ms: u64) -> Vec<StreamItem> {
  let token = CancellationToken::new();
  let (tx, mut rx) = mpsc::channel::<StreamItem>(64);
  let drive_token = token.clone();
  let handle = tokio::spawn(async move {
    drive_typed(source, tx, drive_token).await;
  });
  let mut out = Vec::new();
  let _ = tokio::time::timeout(Duration::from_millis(deadline_ms), async {
    while let Some(item) = rx.recv().await {
      out.push(item);
    }
  })
  .await;
  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
  out
}

fn entry_paths(reset: &StreamItem) -> Vec<String> {
  match reset {
    StreamItem::Reset(r) => r.tasks.iter().map(|t| t.path.clone()).collect(),
    _ => panic!("expected reset, got {reset:?}"),
  }
}

// --- DTO conversion -------------------------------------------------------

#[test]
fn dto_convert_source_entry_projects_id_path_detached() {
  let source = Source::new(4);
  let id = source.register_live("root-1/loop", true);
  let info = source.snapshot().into_iter().find(|t| t.id == id).unwrap();
  let entry = info.to_entry();
  assert_eq!(entry.id, id);
  assert_eq!(entry.path, "root-1/loop");
  assert!(entry.detached);
}

#[test]
fn outcome_str_is_stable_lowercase_for_all_variants() {
  assert_eq!(outcome_str(SourceOutcome::Completed), "completed");
  assert_eq!(outcome_str(SourceOutcome::Cancelled), "cancelled");
  assert_eq!(outcome_str(SourceOutcome::Panicked), "panicked");
}

#[test]
fn sorted_snapshot_orders_by_path_then_id() {
  // Build source entries with local ids, then take a free-standing snapshot.
  let source = Source::new(8);
  let b_id = source.register_live("b", false);
  let a1_id = source.register_live("a", false);
  let a2_id = source.register_live("a", true);
  let snap = source.snapshot();
  // Sanity: ids differ.
  assert_ne!(a1_id, a2_id);
  let out = sorted_snapshot(snap);
  assert_eq!(out[0].path, "a");
  assert_eq!(out[0].id, a2_id);
  assert_eq!(out[1].path, "a");
  assert_eq!(out[1].id, a1_id);
  assert_eq!(out[2].path, "b");
  assert_eq!(out[2].id, b_id);
  // Suppress unused-import: SourceTaskEntry is referenced via sorted_snapshot's input type.
  let _: fn(&[SourceTaskEntry]) = |_| {};
}

// --- Bootstrap reconciliation ---------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_reset_contains_current_snapshot() {
  let source = Source::new(16);
  source.register_live("root-1/loop", false);
  source.register_live("root-1/io", true);

  let items = collect_items(source, 300).await;
  assert_eq!(items.len(), 1, "expected exactly the reset, got {items:?}");
  let tasks = match &items[0] {
    StreamItem::Reset(r) => &r.tasks,
    other => panic!("expected reset, got {other:?}"),
  };
  assert_eq!(tasks.len(), 2);
  // Sorted by path.
  assert_eq!(tasks[0].path, "root-1/io");
  assert_eq!(tasks[1].path, "root-1/loop");
  // Detached flag preserved.
  assert!(tasks[0].detached);
  assert!(!tasks[1].detached);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_subscribes_before_snapshot() {
  // Hold the snapshot gate; while held, the driver has subscribed and blocks on
  // snapshot. We queue a Started event via the broadcast channel, which reaches
  // that receiver. After release, the reset must include the racing start,
  // proving the event was queued between subscribe and snapshot completion.
  let source = Source::new(16);
  source.register_live("root-1/present", false);

  source.hold_snapshot();
  let driver_source = source.clone();
  let handle = tokio::spawn(async move { collect_items(driver_source, 500).await });

  tokio::time::sleep(Duration::from_millis(80)).await;

  // Queue a start that races with the snapshot, WITHOUT adding it to live
  // state, so the snapshot alone would not include it. The stream's drain must
  // fold this event into the live set.
  let late_id = source.register_live("root-1/late", false);
  // Remove it from live state so only the broadcast carries it.
  source.remove_live(late_id);
  source.broadcast_started(late_id, "root-1/late", false);
  source.release_snapshot();

  let items = handle.await.unwrap();
  let paths = entry_paths(&items[0]);
  assert!(
    paths.contains(&"root-1/late".to_owned()),
    "racing start lost: {paths:?}"
  );
  assert!(
    paths.contains(&"root-1/present".to_owned()),
    "pre-existing task lost: {paths:?}"
  );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_preserves_racing_ending_for_session_log() {
  let source = Source::new(16);
  let id = source.register_live("root-1/short", false);

  source.hold_snapshot();
  let driver_source = source.clone();
  let handle = tokio::spawn(async move { collect_items(driver_source, 500).await });

  tokio::time::sleep(Duration::from_millis(80)).await;
  // The task ends while the snapshot gate is held; this Ended is queued.
  source.send_ended(id, "root-1/short", SourceOutcome::Cancelled);
  source.release_snapshot();

  let items = handle.await.unwrap();
  // First: reset with empty live set (task ended before drain completed).
  let tasks = match &items[0] {
    StreamItem::Reset(r) => &r.tasks,
    other => panic!("expected reset first, got {other:?}"),
  };
  assert!(tasks.is_empty(), "ended task should not be live in reset");
  // Second: the racing ending preserved for session history.
  assert!(items.len() >= 2, "expected an ended item, got {items:?}");
  match &items[1] {
    StreamItem::Ended(e) => {
      assert_eq!(e.outcome, "cancelled");
      assert_eq!(e.path, "root-1/short");
    }
    other => panic!("expected ended item, got {other:?}"),
  }
}

// --- Lag recovery ---------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bootstrap_lag_emits_authoritative_reset() {
  // Capacity 2: send >2 events while the snapshot gate is held so the receiver
  // buffer overflows and try_recv returns Lagged during drain. drive_typed must
  // retry and still emit a correct reset.
  let source = Source::new(2);
  source.register_live("root-1/keeper", false);

  source.hold_snapshot();
  let driver_source = source.clone();
  let handle = tokio::spawn(async move { collect_items(driver_source, 600).await });

  tokio::time::sleep(Duration::from_millis(80)).await;
  // Overflow the capacity-2 receiver.
  for i in 0..5usize {
    let id = source.register_live(&format!("root-1/n{i}"), false);
    source.send_started(id, &format!("root-1/n{i}"), false);
  }
  source.release_snapshot();

  let items = handle.await.unwrap();
  // Despite lag during the first drain attempt, the first emitted item must be
  // an authoritative reset.
  let paths = entry_paths(&items[0]);
  assert!(
    paths.contains(&"root-1/keeper".to_owned()),
    "lag recovery dropped keeper: {paths:?}"
  );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steady_stream_forwards_started_and_ended() {
  let source = Source::new(16);
  source.register_live("root-1/long", false);

  let s = source.clone();
  let handle = tokio::spawn(async move { collect_items(s, 500).await });

  // Let the reset land.
  tokio::time::sleep(Duration::from_millis(120)).await;
  let id = source.register_live("root-1/new", false);
  source.send_started(id, "root-1/new", false);
  tokio::time::sleep(Duration::from_millis(100)).await;
  source.send_ended(id, "root-1/new", SourceOutcome::Completed);

  let items = handle.await.unwrap();
  let mut saw_started = false;
  let mut saw_ended = false;
  for item in &items {
    match item {
      StreamItem::Started(s) if s.path == "root-1/new" => saw_started = true,
      StreamItem::Ended(e) if e.path == "root-1/new" => {
        saw_ended = true;
        assert_eq!(e.outcome, "completed");
      }
      _ => {}
    }
  }
  assert!(saw_started, "no started for root-1/new: {items:?}");
  assert!(saw_ended, "no ended for root-1/new: {items:?}");
}

// --- Cancellation ---------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn driver_terminates_on_cancellation_even_with_open_consumer() {
  // The driver must stop promptly when its cancellation token fires, even while
  // a long-lived task keeps the registry emitting events. This is what lets the
  // SSE stream (and thus Axum graceful shutdown) complete instead of hanging.
  let source = Source::new(16);
  source.register_live("root-1/forever", false);

  let token = CancellationToken::new();
  let (tx, _rx_keep_open) = mpsc::channel::<StreamItem>(64);
  // Keep the consumer receiver alive (but never drain) so the only way the
  // driver exits is via cancellation.
  let drive_token = token.clone();
  let tx_for_driver = tx.clone();
  let handle = tokio::spawn(async move {
    drive_typed(source, tx_for_driver, drive_token).await;
  });

  // Let bootstrap (reset) land, so the driver is in the steady-stream loop
  // blocked on rx.recv() with an open consumer.
  tokio::time::sleep(Duration::from_millis(150)).await;
  assert!(
    !handle.is_finished(),
    "driver should still be running before cancellation"
  );

  token.cancel();
  tokio::time::timeout(Duration::from_millis(500), handle)
    .await
    .expect("driver did not terminate within 500ms of cancellation")
    .expect("join error");
}

// --- Steady-state lag recovery preserves freshly-drained endings -----------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn steady_state_lag_recovery_emits_reset_then_drained_endings() {
  // Capacity 2. After bootstrap, overflow the receiver so steady streaming hits
  // Lagged. The recovery must: re-subscribe, snapshot, drain queued events, emit
  // a fresh reset, then emit any endings observed after the new subscription
  // boundary (in order). Lag-lost endings are unrecoverable.
  let source = Source::new(2);
  // One persistent task present in every snapshot.
  let keeper = source.register_live("root-1/keeper", false);

  let token = CancellationToken::new();
  let (tx, mut rx) = mpsc::channel::<StreamItem>(64);
  let drive_token = token.clone();
  let driver_source = source.clone();
  let handle = tokio::spawn(async move {
    drive_typed(driver_source, tx, drive_token).await;
  });

  // Wait for bootstrap reset to land.
  let first = tokio::time::timeout(Duration::from_millis(300), rx.recv())
    .await
    .expect("no bootstrap reset")
    .expect("channel closed");
  assert!(matches!(first, StreamItem::Reset(_)));

  // Now overflow the capacity-2 receiver in steady state. To force a Lagged
  // that is then *followed* by a cleanly-drained ending, we:
  //   1. hold the snapshot gate (so the recovery's snapshot blocks),
  //   2. overflow the receiver (> capacity) with started events -> Lagged,
  //   3. then send exactly one ending AFTER the fresh subscribe (so the
  //      recovery drain sees it and must replay it after the reset),
  //   4. release the gate.
  source.hold_snapshot();
  tokio::time::sleep(Duration::from_millis(80)).await;

  // Step 2: overflow.
  for i in 0..5usize {
    let id = source.register_live(&format!("root-1/noise-{i}"), false);
    source.send_started(id, &format!("root-1/noise-{i}"), false);
  }

  // Give the steady loop a moment to observe the Lagged and begin recovery
  // (which re-subscribes, then blocks on the held snapshot gate).
  tokio::time::sleep(Duration::from_millis(120)).await;

  // Step 3: an ending observed AFTER the fresh subscribe/drain boundary. It is
  // in the new receiver's buffer and must be drained + replayed after the reset.
  source.send_ended(keeper, "root-1/keeper", SourceOutcome::Panicked);

  // Step 4: release the snapshot so recovery completes.
  source.release_snapshot();

  // Collect subsequent items with a deadline.
  let mut items = Vec::new();
  let _ = tokio::time::timeout(Duration::from_millis(400), async {
    while let Some(item) = rx.recv().await {
      items.push(item);
    }
  })
  .await;
  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;

  // The first recovery item must be a reset (authoritative live-set repair).
  let reset_idx = items
    .iter()
    .position(|i| matches!(i, StreamItem::Reset(_)))
    .expect("expected a recovery reset");
  // After the reset, the freshly-drained ending (root-1/keeper, panicked) must
  // appear, proving lag recovery does not discard post-boundary endings.
  let after_reset = &items[reset_idx + 1..];
  let saw_keeper_panicked = after_reset.iter().any(|i| match i {
    StreamItem::Ended(e) => e.path == "root-1/keeper" && e.outcome == "panicked",
    _ => false,
  });
  assert!(
    saw_keeper_panicked,
    "recovery must replay the freshly-drained ending after reset: {after_reset:?}"
  );
}

// Keep the TaskEventSource trait import honest.
#[allow(dead_code)]
fn _trait_used<S: TaskEventSource>(_s: &S) {}
