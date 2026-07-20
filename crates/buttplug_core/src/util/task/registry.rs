// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use dashmap::DashMap;
use std::sync::{
  Mutex,
  OnceLock,
  atomic::{AtomicU64, Ordering},
};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// Why a task could not be spawned into its scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TaskSpawnError {
  /// The scope was cancelled before registration could win the race.
  #[error("task scope is closed")]
  ScopeClosed,
}

/// Unique identifier for a registered task. Process-lifetime unique.
///
/// IDs are drawn from a monotonically-incrementing `AtomicU64` counter. At
/// 64-bit width and even at 10 million tasks per second the counter would take
/// roughly 58,000 years to wrap, so id reuse within a single process lifetime
/// is not a practical concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
  pub fn value(&self) -> u64 {
    self.0
  }
}

/// How a task ended: ran to completion on its own, exited after observing
/// cancellation, or unwound from a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskOutcome {
  Completed,
  Cancelled,
  Panicked,
}

/// Registry entry for a live task.
#[derive(Debug, Clone)]
pub struct TaskInfo {
  pub id: TaskId,
  /// Hierarchical path, e.g. "device-manager-1/devices/device-3/io".
  pub path: String,
  /// True if spawned via spawn_detached (no owning scope).
  pub detached: bool,
}

/// Lifecycle events broadcast by the Task Registry.
#[derive(Debug, Clone)]
pub enum TaskEvent {
  Started {
    id: TaskId,
    path: String,
  },
  Ended {
    id: TaskId,
    path: String,
    outcome: TaskOutcome,
  },
}

/// The global record of every live task, populated as a side effect of
/// spawning through a Task Scope.
///
/// **Memory note**: entries are removed on `deregister`, but `DashMap` does not
/// shrink shard capacity after removals. Peak concurrent-task count therefore
/// becomes a memory high-water mark that is held for the lifetime of the
/// registry (i.e. the process). In practice buttplug servers run a bounded
/// number of concurrent tasks, so this is not expected to be significant.
#[derive(Debug)]
pub struct TaskRegistry {
  tasks: DashMap<u64, TaskInfo>,
  /// Short-held gate ordering registration against scope cancellation.
  gate: Mutex<()>,
  counter: AtomicU64,
  root_counter: AtomicU64,
  events: broadcast::Sender<TaskEvent>,
}

/// The global Task Registry.
pub fn registry() -> &'static TaskRegistry {
  static REGISTRY: OnceLock<TaskRegistry> = OnceLock::new();
  REGISTRY.get_or_init(TaskRegistry::new)
}

impl TaskRegistry {
  pub(super) fn new() -> Self {
    Self {
      tasks: DashMap::new(),
      gate: Mutex::new(()),
      counter: AtomicU64::new(1),
      root_counter: AtomicU64::new(1),
      events: broadcast::channel(256).0,
    }
  }

  /// Unique suffix for root scope names so parallel instances don't collide.
  pub(super) fn next_root_suffix(&self) -> u64 {
    self.root_counter.fetch_add(1, Ordering::Relaxed)
  }

  pub(super) fn register(&self, path: String, detached: bool) -> TaskId {
    let _gate = self
      .gate
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    self.register_locked(path, detached)
  }

  pub(super) fn register_scoped(
    &self,
    path: String,
    token: &CancellationToken,
  ) -> Result<(TaskId, CancellationToken), TaskSpawnError> {
    let _gate = self
      .gate
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if token.is_cancelled() {
      return Err(TaskSpawnError::ScopeClosed);
    }
    let task_token = token.child_token();
    let id = self.register_locked(path, false);
    Ok((id, task_token))
  }

  pub(super) fn cancel(&self, token: &CancellationToken) {
    let _gate = self
      .gate
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    token.cancel();
  }

  #[cfg(test)]
  pub(crate) fn test_hold_gate(&self) -> std::sync::MutexGuard<'_, ()> {
    self
      .gate
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  fn register_locked(&self, path: String, detached: bool) -> TaskId {
    let id = TaskId(self.counter.fetch_add(1, Ordering::Relaxed));
    self.tasks.insert(
      id.0,
      TaskInfo {
        id,
        path: path.clone(),
        detached,
      },
    );
    let _ = self.events.send(TaskEvent::Started { id, path });
    id
  }

  pub(super) fn deregister(&self, id: TaskId, outcome: TaskOutcome) {
    if let Some((_, info)) = self.tasks.remove(&id.0) {
      let _ = self.events.send(TaskEvent::Ended {
        id,
        path: info.path,
        outcome,
      });
    }
  }

  /// Snapshot of all live tasks.
  pub fn snapshot(&self) -> Vec<TaskInfo> {
    self.tasks.iter().map(|e| e.value().clone()).collect()
  }

  /// Count of live tasks at or under the given path prefix. Prefix matching is
  /// segment-aware: "server-1" matches "server-1/loop" but not "server-10/loop".
  pub fn live_count_under(&self, prefix: &str) -> usize {
    self
      .tasks
      .iter()
      .filter(|e| path_is_under(&e.value().path, prefix))
      .count()
  }

  /// Subscribe to task lifecycle events.
  pub fn event_stream(&self) -> broadcast::Receiver<TaskEvent> {
    self.events.subscribe()
  }

  /// Wait until no live tasks remain at or under the given path prefix.
  /// Subscribes to events BEFORE counting to avoid missing an Ended event
  /// between the count and the wait. Callers should wrap in a timeout if the
  /// subtree may contain uncooperative tasks.
  pub async fn wait_empty_under(&self, prefix: &str) {
    let mut events = self.events.subscribe();
    while self.live_count_under(prefix) > 0 {
      match events.recv().await {
        Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => continue,
        // Sender is owned by the registry itself; Closed only happens for
        // non-global registries in tests being dropped.
        Err(broadcast::error::RecvError::Closed) => return,
      }
    }
  }
}

fn path_is_under(path: &str, prefix: &str) -> bool {
  path == prefix || (path.starts_with(prefix) && path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_register_and_snapshot() {
    let reg = TaskRegistry::new();
    let id = reg.register("root-1/loop".to_owned(), false);
    let snapshot = reg.snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id, id);
    assert_eq!(snapshot[0].path, "root-1/loop");
    reg.deregister(id, TaskOutcome::Completed);
    assert!(reg.snapshot().is_empty());
  }

  #[test]
  fn test_prefix_boundary() {
    let reg = TaskRegistry::new();
    reg.register("server-1/loop".to_owned(), false);
    reg.register("server-10/loop".to_owned(), false);
    // "server-1" must not match "server-10/loop"
    assert_eq!(reg.live_count_under("server-1"), 1);
    assert_eq!(reg.live_count_under("server-10"), 1);
    assert_eq!(reg.live_count_under("server"), 0);
  }

  #[tokio::test]
  async fn test_events_emitted() {
    let reg = TaskRegistry::new();
    let mut events = reg.event_stream();
    let id = reg.register("root-1/task".to_owned(), false);
    reg.deregister(id, TaskOutcome::Cancelled);
    assert!(matches!(
      events.recv().await.unwrap(),
      TaskEvent::Started { .. }
    ));
    let TaskEvent::Ended { outcome, .. } = events.recv().await.unwrap() else {
      panic!("expected Ended event");
    };
    assert_eq!(outcome, TaskOutcome::Cancelled);
  }

  #[tokio::test]
  async fn test_wait_empty_under() {
    let reg = std::sync::Arc::new(TaskRegistry::new());
    let id = reg.register("root-1/task".to_owned(), false);
    let reg_clone = reg.clone();
    let waiter = tokio::spawn(async move { reg_clone.wait_empty_under("root-1").await });
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert!(!waiter.is_finished());
    reg.deregister(id, TaskOutcome::Completed);
    tokio::time::timeout(std::time::Duration::from_secs(1), waiter)
      .await
      .expect("wait_empty_under did not resolve")
      .unwrap();
  }
}
