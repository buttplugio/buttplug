// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Wire DTOs, the engine-local [`TaskEventSource`] abstraction, subscribe-before-
//! snapshot bootstrap reconciliation, and lag recovery for the diagnostics
//! stream.
//!
//! This module is the boundary between diagnostics-owned source records and the
//! wire contract. Source records ([`SourceTaskEntry`], [`SourceTaskEvent`],
//! [`SourceOutcome`]) use raw `u64` task IDs and carry no `buttplug_core` types.
//! The production adapter ([`RegistryTaskEventSource`]) converts core
//! [`TaskEvent`]/[`TaskInfo`] into these source records at its boundary; the
//! scripted test source constructs them directly with a local ID counter and
//! never touches the process-global registry.
//!
//! The wire contract uses serializable diagnostics-specific DTOs with stable
//! lowercase event/outcome strings. Conversion from source records to wire DTOs
//! is centralized in [`DtoConvert`] so task fields and outcome spelling cannot
//! diverge across the snapshot and stream paths.

use std::collections::BTreeMap;

use axum::response::sse::Event;
use buttplug_core::util::task::{TaskEvent, TaskOutcome, registry};
use futures::stream::Stream;
use serde::Serialize;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Diagnostics-owned source records (raw u64 IDs; no buttplug_core types)
// ---------------------------------------------------------------------------

/// Diagnostics-owned projection of one live task at the source boundary. Uses a
/// raw `u64` id instead of [`buttplug_core::util::task::TaskId`] so the
/// [`TaskEventSource`] abstraction and the scripted test source never touch the
/// core registry. The production adapter converts [`TaskInfo`] into this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTaskEntry {
  pub id: u64,
  pub path: String,
  pub detached: bool,
}

/// Diagnostics-owned lifecycle outcome, mirroring
/// [`buttplug_core::util::task::TaskOutcome`] without exposing it through the
/// source abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceOutcome {
  Completed,
  Cancelled,
  Panicked,
}

/// Convert a core [`TaskOutcome`] into the diagnostics-owned [`SourceOutcome`].
/// Used only at the production adapter boundary.
impl From<TaskOutcome> for SourceOutcome {
  fn from(outcome: TaskOutcome) -> Self {
    match outcome {
      TaskOutcome::Completed => SourceOutcome::Completed,
      TaskOutcome::Cancelled => SourceOutcome::Cancelled,
      TaskOutcome::Panicked => SourceOutcome::Panicked,
    }
  }
}

/// Diagnostics-owned lifecycle event at the source boundary, mirroring
/// [`TaskEvent`] with raw `u64` ids. The production adapter converts core
/// [`TaskEvent`] into this; the scripted source emits it directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceTaskEvent {
  Started {
    id: u64,
    path: String,
    detached: bool,
  },
  Ended {
    id: u64,
    path: String,
    outcome: SourceOutcome,
  },
}

// ---------------------------------------------------------------------------
// Receiver: hides whether the underlying channel carries SourceTaskEvent
// (scripted) or TaskEvent (production, converted inline at the boundary)
// ---------------------------------------------------------------------------

/// Receiver for diagnostics lifecycle events.
///
/// The [`Direct`](TaskEventReceiver::Direct) variant wraps a channel of
/// [`SourceTaskEvent`] (used by the scripted test source). The
/// [`Converting`](TaskEventReceiver::Converting) variant wraps the core
/// registry's `broadcast::Receiver<TaskEvent>` and converts each event inline as
/// it is drained. Conversion introduces no async hop, so it cannot race with the
/// subscribe-before-snapshot contract: the registry subscription is established
/// synchronously inside [`TaskEventSource::subscribe`], and every subsequent
/// `try_recv`/`recv` returns an already-converted [`SourceTaskEvent`].
pub enum TaskEventReceiver {
  /// Backed by a private channel of diagnostics-owned [`SourceTaskEvent`].
  Direct(broadcast::Receiver<SourceTaskEvent>),
  /// Backed by the core registry channel; events are converted inline.
  Converting(broadcast::Receiver<TaskEvent>),
}

impl TaskEventReceiver {
  /// Non-blocking receive; mirrors `broadcast::Receiver::try_recv`. Core events
  /// are converted to [`SourceTaskEvent`] inline, and the original
  /// [`broadcast::error::TryRecvError`] (including `Lagged`) is preserved so the
  /// lag-recovery contract is unaffected.
  pub fn try_recv(&mut self) -> Result<SourceTaskEvent, broadcast::error::TryRecvError> {
    match self {
      TaskEventReceiver::Direct(rx) => rx.try_recv(),
      TaskEventReceiver::Converting(rx) => rx.try_recv().map(convert_event),
    }
  }

  /// Async receive; mirrors `broadcast::Receiver::recv`. Core events are
  /// converted to [`SourceTaskEvent`] inline, and the original
  /// [`broadcast::error::RecvError`] (including `Lagged`) is preserved so the
  /// lag-recovery contract is unaffected.
  pub async fn recv(&mut self) -> Result<SourceTaskEvent, broadcast::error::RecvError> {
    match self {
      TaskEventReceiver::Direct(rx) => rx.recv().await,
      TaskEventReceiver::Converting(rx) => rx.recv().await.map(convert_event),
    }
  }
}

/// Convert a core [`TaskEvent`] into the diagnostics-owned [`SourceTaskEvent`].
/// Used only at the production adapter boundary.
fn convert_event(ev: TaskEvent) -> SourceTaskEvent {
  match ev {
    TaskEvent::Started { id, path, detached } => SourceTaskEvent::Started {
      id: id.value(),
      path,
      detached,
    },
    TaskEvent::Ended { id, path, outcome } => SourceTaskEvent::Ended {
      id: id.value(),
      path,
      outcome: outcome.into(),
    },
  }
}

// ---------------------------------------------------------------------------
// Wire DTOs
// ---------------------------------------------------------------------------

/// One live task as presented to the browser: a stable, minimal projection of
/// [`SourceTaskEntry`]. Fields are exactly the only data the registry retains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskDiagnosticsEntry {
  pub id: u64,
  pub path: String,
  pub detached: bool,
}

/// Data carried by a `reset` SSE event: the reconciled authoritative live set,
/// sorted deterministically.
#[derive(Debug, Clone, Serialize)]
pub struct ResetEvent {
  pub tasks: Vec<TaskDiagnosticsEntry>,
}

/// Data carried by a `started` SSE event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StartedEvent {
  pub id: u64,
  pub path: String,
  pub detached: bool,
}

/// Data carried by an `ended` SSE event. `outcome` is one of the stable
/// lowercase strings `"completed"`, `"cancelled"`, `"panicked"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EndedEvent {
  pub id: u64,
  pub path: String,
  pub outcome: &'static str,
}

/// Centralized conversion so the snapshot and stream paths can never diverge on
/// field selection or outcome spelling.
pub trait DtoConvert {
  fn to_entry(&self) -> TaskDiagnosticsEntry;
}

impl DtoConvert for SourceTaskEntry {
  fn to_entry(&self) -> TaskDiagnosticsEntry {
    TaskDiagnosticsEntry {
      id: self.id,
      path: self.path.clone(),
      detached: self.detached,
    }
  }
}

/// Stable lowercase spelling of a [`SourceOutcome`]. Single source of truth for
/// the wire contract.
pub(crate) fn outcome_str(outcome: SourceOutcome) -> &'static str {
  match outcome {
    SourceOutcome::Completed => "completed",
    SourceOutcome::Cancelled => "cancelled",
    SourceOutcome::Panicked => "panicked",
  }
}

/// Sort a snapshot deterministically (path, then id) so `DashMap` iteration
/// order cannot make tests or clients flaky.
pub(crate) fn sorted_snapshot(snapshot: Vec<SourceTaskEntry>) -> Vec<TaskDiagnosticsEntry> {
  let mut entries: Vec<TaskDiagnosticsEntry> = snapshot.iter().map(DtoConvert::to_entry).collect();
  entries.sort_by(|a, b| a.path.cmp(&b.path).then(a.id.cmp(&b.id)));
  entries
}

/// Build an SSE [`Event`] with a typed JSON payload.
fn sse_event<E: Serialize>(name: &'static str, payload: &E) -> Event {
  let data = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_owned());
  Event::default().event(name).data(data)
}

// ---------------------------------------------------------------------------
// TaskEventSource abstraction
// ---------------------------------------------------------------------------

/// Error returned by a [`TaskEventSource`]. `Lagged` indicates the broadcast
/// receiver fell behind and the caller should re-snapshot; `Closed` means the
/// underlying sender is gone (the global registry's sender never closes in
/// production, but scripted/test sources may).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskEventSourceError {
  Lagged(u64),
  Closed,
}

/// The operations the diagnostics stream needs from the registry, and nothing
/// more. The production implementation ([`RegistryTaskEventSource`]) converts
/// core registry snapshot entries and stream events into diagnostics-owned
/// source records ([`SourceTaskEntry`] / [`SourceTaskEvent`]) at this boundary,
/// so the rest of the diagnostics module never sees `buttplug_core` types. Tests
/// use a scripted source that constructs source records directly from a local
/// counter.
///
/// This trait intentionally does not expose the registry's private
/// register/deregister methods, so it cannot be used to mutate registry state.
pub trait TaskEventSource: Send {
  /// Subscribe to lifecycle events. MUST be obtainable before [`snapshot`].
  /// Establishes the subscription synchronously before returning so the
  /// subscribe-before-snapshot contract holds regardless of the receiver variant.
  fn subscribe(&self) -> TaskEventReceiver;

  /// Snapshot of all live tasks, as diagnostics-owned [`SourceTaskEntry`].
  /// Iteration order is unspecified; callers sort.
  fn snapshot(&self) -> Vec<SourceTaskEntry>;
}

/// Production adapter over the process-global registry. Converts core
/// [`TaskInfo`] snapshot entries and [`TaskEvent`] stream events into
/// diagnostics-owned source records ([`SourceTaskEntry`] / [`SourceTaskEvent`])
/// at this boundary, so the rest of the diagnostics module operates only on
/// diagnostics-owned types with raw `u64` ids.
#[derive(Debug, Clone, Copy)]
pub struct RegistryTaskEventSource;

impl TaskEventSource for RegistryTaskEventSource {
  fn subscribe(&self) -> TaskEventReceiver {
    // Synchronous: the registry subscription is established before this
    // returns, so events queued between subscribe and snapshot are captured.
    TaskEventReceiver::Converting(registry().event_stream())
  }

  fn snapshot(&self) -> Vec<SourceTaskEntry> {
    registry()
      .snapshot()
      .into_iter()
      .map(|t| SourceTaskEntry {
        id: t.id.value(),
        path: t.path,
        detached: t.detached,
      })
      .collect()
  }
}

pub(crate) fn registry_source() -> RegistryTaskEventSource {
  RegistryTaskEventSource
}

// ---------------------------------------------------------------------------
// Bootstrap reconciliation + steady streaming
// ---------------------------------------------------------------------------

/// A single item produced by the diagnostics stream, ready to be turned into an
/// SSE `Event`.
#[derive(Debug)]
pub(crate) enum StreamItem {
  Reset(ResetEvent),
  Started(StartedEvent),
  Ended(EndedEvent),
}

impl StreamItem {
  fn into_sse(self) -> Event {
    match self {
      StreamItem::Reset(e) => sse_event("reset", &e),
      StreamItem::Started(e) => sse_event("started", &e),
      StreamItem::Ended(e) => sse_event("ended", &e),
    }
  }
}

/// Reconcile a fresh snapshot against an event receiver, returning the live set
/// and the endings observed during drain. Returns `Err(())` if the receiver
/// lagged during drain, signalling the caller to retry from a new snapshot.
///
/// The receiver is the diagnostics-owned [`TaskEventReceiver`]; core events have
/// already been converted to [`SourceTaskEvent`] by the time they reach here.
fn reconcile<S: TaskEventSource>(
  source: &S,
  rx: &mut TaskEventReceiver,
) -> Result<(Vec<TaskDiagnosticsEntry>, Vec<EndedEvent>), ()> {
  let snapshot = source.snapshot();
  let mut live: BTreeMap<u64, TaskDiagnosticsEntry> = snapshot
    .iter()
    .map(DtoConvert::to_entry)
    .map(|e| (e.id, e))
    .collect();

  let mut drained_endings: Vec<EndedEvent> = Vec::new();

  loop {
    match rx.try_recv() {
      Ok(SourceTaskEvent::Started { id, path, detached }) => {
        live.insert(id, TaskDiagnosticsEntry { id, path, detached });
      }
      Ok(SourceTaskEvent::Ended { id, path, outcome }) => {
        live.remove(&id);
        drained_endings.push(EndedEvent {
          id,
          path,
          outcome: outcome_str(outcome),
        });
      }
      Err(broadcast::error::TryRecvError::Empty) | Err(broadcast::error::TryRecvError::Closed) => {
        break;
      }
      Err(broadcast::error::TryRecvError::Lagged(_)) => return Err(()),
    }
  }

  let mut tasks: Vec<TaskDiagnosticsEntry> = live.into_values().collect();
  tasks.sort_by(|a, b| a.path.cmp(&b.path).then(a.id.cmp(&b.id)));
  Ok((tasks, drained_endings))
}

/// Drive the bootstrap + steady-stream loop, pushing typed [`StreamItem`]s into
/// `tx`. Every `.await` point races against `shutdown` so the stream terminates
/// promptly when the server is cancelled (Axum graceful shutdown alone would
/// wait forever for an open SSE connection).
///
/// Ordering/lag contract:
/// - subscribe *before* snapshot, then non-blockingly drain queued events;
/// - emit one authoritative `reset`, then replay drained endings in order;
/// - on steady-state lag, re-subscribe + snapshot + drain, emit a fresh `reset`,
///   then replay the *freshly drained* endings (those observed after the new
///   subscription boundary) in order. Only genuinely lag-lost endings (those the
///   broadcast channel dropped between the overflow and the fresh subscription)
///   are unrecoverable.
pub(crate) async fn drive_typed<S: TaskEventSource>(
  source: S,
  tx: mpsc::Sender<StreamItem>,
  shutdown: CancellationToken,
) {
  // Helper: send an item, but bail on cancellation or a dropped consumer.
  macro_rules! send_item {
    ($tx:expr, $item:expr) => {{
      tokio::select! {
        biased;
        _ = shutdown.cancelled() => return,
        res = $tx.send($item) => { if res.is_err() { return; } }
      }
    }};
  }

  // --- Bootstrap: loop until an authoritative reset can be emitted ---------
  let mut rx = source.subscribe();
  loop {
    if shutdown.is_cancelled() {
      return;
    }
    match reconcile(&source, &mut rx) {
      Ok((tasks, drained_endings)) => {
        send_item!(tx, StreamItem::Reset(ResetEvent { tasks }));
        for ended in drained_endings {
          send_item!(tx, StreamItem::Ended(ended));
        }
        break;
      }
      Err(()) => {
        // Lagged during drain: re-subscribe and retry with a fresh snapshot so
        // the live set is correct. Lag-lost endings are unrecoverable. Yield
        // before retrying so sustained churn cannot starve a current-thread
        // Tokio runtime.
        tokio::task::yield_now().await;
        rx = source.subscribe();
      }
    }
  }

  // --- Steady streaming ----------------------------------------------------
  loop {
    // Race the next event against cancellation.
    let next = tokio::select! {
      biased;
      _ = shutdown.cancelled() => return,
      r = rx.recv() => r,
    };

    match next {
      Ok(SourceTaskEvent::Started { id, path, detached }) => {
        send_item!(tx, StreamItem::Started(StartedEvent { id, path, detached }));
      }
      Ok(SourceTaskEvent::Ended { id, path, outcome }) => {
        send_item!(
          tx,
          StreamItem::Ended(EndedEvent {
            id,
            path,
            outcome: outcome_str(outcome),
          })
        );
      }
      Err(broadcast::error::RecvError::Lagged(_)) => {
        // Re-snapshot and reconcile exactly as in bootstrap. Endings drained
        // after the fresh subscription boundary MUST be emitted after the reset
        // (in original order) so the browser-session history retains them; only
        // endings lost to the overflow itself are unrecoverable.
        loop {
          if shutdown.is_cancelled() {
            return;
          }
          let mut fresh = source.subscribe();
          match reconcile(&source, &mut fresh) {
            Ok((tasks, drained_endings)) => {
              send_item!(tx, StreamItem::Reset(ResetEvent { tasks }));
              for ended in drained_endings {
                send_item!(tx, StreamItem::Ended(ended));
              }
              rx = fresh;
              break;
            }
            Err(()) => {
              // Lagged again during the recovery drain; yield before retrying
              // from a newer subscription + snapshot so sustained churn cannot
              // monopolize a current-thread Tokio runtime.
              tokio::task::yield_now().await;
              continue;
            }
          }
        }
      }
      Err(broadcast::error::RecvError::Closed) => {
        // Global registry sender never closes in production; stop streaming.
        return;
      }
    }
  }
}

/// Drive the stream and forward each item as an SSE `Event`. Production adapter
/// over [`drive_typed`]. Terminates when `shutdown` is cancelled or the
/// consumer (SSE connection) drops.
pub(crate) async fn drive_stream<S: TaskEventSource + 'static>(
  source: S,
  tx: mpsc::Sender<Event>,
  shutdown: CancellationToken,
) {
  let (typed_tx, mut typed_rx) = mpsc::channel::<StreamItem>(64);
  let driver_shutdown = shutdown.clone();
  let driver = tokio::spawn(async move {
    drive_typed(source, typed_tx, driver_shutdown).await;
  });
  loop {
    tokio::select! {
      biased;
      _ = shutdown.cancelled() => break,
      item = typed_rx.recv() => {
        match item {
          Some(item) => {
            if tx.send(item.into_sse()).await.is_err() {
              break;
            }
          }
          None => break,
        }
      }
    }
  }
  driver.abort();
  let _ = driver.await;
}

/// Build the diagnostics event stream for one SSE connection. The stream
/// terminates when `shutdown` is cancelled, which lets Axum's graceful shutdown
/// complete instead of hanging on an open `EventSource` connection. See
/// [`drive_typed`] for the ordering/lag contract.
pub(crate) fn diagnostics_stream<S>(
  source: S,
  shutdown: CancellationToken,
) -> impl Stream<Item = Result<Event, std::convert::Infallible>> + Send
where
  S: TaskEventSource + Send + 'static,
{
  let (tx, rx) = mpsc::channel::<Event>(64);
  tokio::spawn(async move {
    drive_stream(source, tx, shutdown).await;
  });
  ReceiverStream::new(rx).map(Ok)
}

// Bring `StreamExt::map` into scope for the stream returned above.
use futures::StreamExt;
