// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! A fully-isolated scripted [`TaskEventSource`] for unit-testing bootstrap
//! reconciliation and lag recovery.
//!
//! Isolation guarantees: this source allocates ids from a **local**
//! `AtomicU64` counter and uses a private broadcast channel of diagnostics-owned
//! [`SourceTaskEvent`]. It never calls [`buttplug_core::util::task::registry`],
//! `spawn_detached`, `TaskScope`, or any other process-global state or timing.
//! That makes races (subscribe-before-snapshot), lag recovery, and ending
//! preservation fully deterministic without depending on async dispatch or the
//! global registry counter.
//!
//! Lives outside `cfg(test)` so the protocol module's own `#[cfg(test)]` tests
//! can import it.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use tokio::sync::broadcast;

use super::protocol::{
  SourceOutcome, SourceTaskEntry, SourceTaskEvent, TaskEventReceiver, TaskEventSource,
};

/// Scripted source used by the protocol/bootstrap/lag tests.
#[derive(Clone)]
pub struct ScriptedTaskEventSource {
  inner: Arc<Inner>,
}

struct Inner {
  capacity: usize,
  sender: Mutex<Option<broadcast::Sender<SourceTaskEvent>>>,
  live: Mutex<Vec<SourceTaskEntry>>,
  gate: Condvar,
  snapshot_state: Mutex<bool>,
  /// Local id allocator; process-independent, so tests do not depend on the
  /// global registry counter or any other test's task churn.
  next_id: AtomicU64,
}

impl ScriptedTaskEventSource {
  /// Create a scripted source with the given broadcast capacity.
  pub fn new(capacity: usize) -> Self {
    let (tx, _rx) = broadcast::channel(capacity);
    Self {
      inner: Arc::new(Inner {
        capacity,
        sender: Mutex::new(Some(tx)),
        live: Mutex::new(Vec::new()),
        gate: Condvar::new(),
        snapshot_state: Mutex::new(true),
        next_id: AtomicU64::new(1),
      }),
    }
  }

  /// Allocate a fresh local id. Monotonic within this source instance only.
  fn alloc_id(&self) -> u64 {
    self.inner.next_id.fetch_add(1, Ordering::Relaxed)
  }

  /// Hold the snapshot gate so the next `snapshot()` blocks until released.
  pub fn hold_snapshot(&self) {
    let mut state = self.inner.snapshot_state.lock().unwrap();
    *state = false;
  }

  /// Allow a blocked `snapshot()` call to proceed.
  pub fn release_snapshot(&self) {
    let mut state = self.inner.snapshot_state.lock().unwrap();
    *state = true;
    self.inner.gate.notify_all();
  }

  /// Insert a live task into the snapshot contents and return its local id.
  pub fn register_live(&self, path: &str, detached: bool) -> u64 {
    let id = self.alloc_id();
    {
      let mut live = self.inner.live.lock().unwrap();
      live.retain(|t| t.id != id);
      live.push(SourceTaskEntry {
        id,
        path: path.to_owned(),
        detached,
      });
    }
    id
  }

  /// Remove a live task from the snapshot contents.
  pub fn remove_live(&self, id: u64) {
    self.inner.live.lock().unwrap().retain(|t| t.id != id);
  }

  /// Broadcast a `Started` event WITHOUT mutating live contents.
  pub fn broadcast_started(&self, id: u64, path: &str, detached: bool) {
    let _ = self.inner.sender.lock().unwrap().as_ref().map(|s| {
      s.send(SourceTaskEvent::Started {
        id,
        path: path.to_owned(),
        detached,
      })
    });
  }

  /// Broadcast an `Ended` event WITHOUT mutating live contents.
  pub fn broadcast_ended(&self, id: u64, path: &str, outcome: SourceOutcome) {
    let _ = self.inner.sender.lock().unwrap().as_ref().map(|s| {
      s.send(SourceTaskEvent::Ended {
        id,
        path: path.to_owned(),
        outcome,
      })
    });
  }

  /// Broadcast a `Started` event and record it as live.
  pub fn send_started(&self, id: u64, path: &str, detached: bool) {
    {
      let mut live = self.inner.live.lock().unwrap();
      live.retain(|t| t.id != id);
      live.push(SourceTaskEntry {
        id,
        path: path.to_owned(),
        detached,
      });
    }
    self.broadcast_started(id, path, detached);
  }

  /// Broadcast an `Ended` event and remove the task from live contents.
  pub fn send_ended(&self, id: u64, path: &str, outcome: SourceOutcome) {
    self.remove_live(id);
    self.broadcast_ended(id, path, outcome);
  }

  /// Replace the live snapshot contents directly.
  pub fn set_live(&self, live: Vec<SourceTaskEntry>) {
    *self.inner.live.lock().unwrap() = live;
  }
}

impl TaskEventSource for ScriptedTaskEventSource {
  fn subscribe(&self) -> TaskEventReceiver {
    // Recreate the channel so a new subscriber starts with an empty buffer.
    let mut guard = self.inner.sender.lock().unwrap();
    let (tx, rx) = broadcast::channel(self.inner.capacity);
    *guard = Some(tx);
    TaskEventReceiver::Direct(rx)
  }

  fn snapshot(&self) -> Vec<SourceTaskEntry> {
    let mut state = self.inner.snapshot_state.lock().unwrap();
    while !*state {
      state = self.inner.gate.wait(state).unwrap();
    }
    self.inner.live.lock().unwrap().clone()
  }
}
