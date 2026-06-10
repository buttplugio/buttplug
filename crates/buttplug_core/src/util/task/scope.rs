// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::registry::{TaskId, TaskOutcome, registry};
use crate::util::async_manager;
use std::future::Future;
use tokio_util::sync::CancellationToken;

/// The owner of spawned async tasks within a module.
///
/// Tasks can only be spawned through a scope, which derives their hierarchical
/// path, registers them in the global Task Registry, and hands them a
/// cooperative [CancellationToken]. Dropping a scope cancels its subtree.
///
/// Scopes are intentionally NOT [Clone]: ownership is singular. To share
/// spawning capability, create a [child][TaskScope::child] and move it, or
/// wrap a scope in [std::sync::Arc] when a cloneable holder needs it (the
/// subtree then cancels when the last Arc drops).
#[derive(Debug)]
pub struct TaskScope {
  path: String,
  token: CancellationToken,
}

impl TaskScope {
  /// Create a root scope. The path gets a unique numeric suffix
  /// (e.g. "server-2") so parallel instances in one process don't collide.
  pub fn root(name: &str) -> Self {
    Self {
      path: format!("{}-{}", name, registry().next_root_suffix()),
      token: CancellationToken::new(),
    }
  }

  /// Create a child scope. Cancelling this scope cancels the child.
  pub fn child(&self, name: &str) -> Self {
    Self {
      path: format!("{}/{}", self.path, name),
      token: self.token.child_token(),
    }
  }

  pub fn path(&self) -> &str {
    &self.path
  }

  /// The scope's own cancellation token, for select!-ing in code that runs
  /// inside an already-spawned context.
  pub fn token(&self) -> &CancellationToken {
    &self.token
  }

  /// Request cancellation of every task in this scope's subtree.
  pub fn cancel(&self) {
    self.token.cancel();
  }

  /// Cancel the subtree and wait until every task under this scope has
  /// deregistered. Wrap in a timeout if the subtree may contain
  /// uncooperative tasks.
  pub async fn shutdown(&self) {
    self.token.cancel();
    registry().wait_empty_under(&self.path).await;
  }

  /// Spawn a task owned by this scope. The closure receives the task's own
  /// child token; long-running tasks MUST select on it.
  #[cfg(not(feature = "wasm"))]
  pub fn spawn<F, Fut>(&self, name: &str, f: F)
  where
    F: FnOnce(CancellationToken) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
  {
    let (id, task_token, span) = self.register_task(name);
    let fut = f(task_token.clone());
    async_manager::spawn(finish_task(fut, id, task_token), span);
  }

  /// Spawn a task owned by this scope (WASM, no Send required).
  #[cfg(feature = "wasm")]
  pub fn spawn<F, Fut>(&self, name: &str, f: F)
  where
    F: FnOnce(CancellationToken) -> Fut,
    Fut: Future<Output = ()> + 'static,
  {
    let (id, task_token, span) = self.register_task(name);
    let fut = f(task_token.clone());
    async_manager::spawn(finish_task(fut, id, task_token), span);
  }

  /// Consume the scope and spawn a task that holds it alive for its own
  /// duration. Use when the caller has nowhere to store the scope (e.g.
  /// protocol subscription handlers): drop-cancel must not fire before the
  /// task runs, but parent cancellation still propagates.
  #[cfg(not(feature = "wasm"))]
  pub fn spawn_and_hold<F, Fut>(self, name: &str, f: F)
  where
    F: FnOnce(CancellationToken) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
  {
    let (id, task_token, span) = self.register_task(name);
    let fut = f(task_token.clone());
    async_manager::spawn(
      async move {
        let _hold = self;
        finish_task(fut, id, task_token).await;
      },
      span,
    );
  }

  /// Consume the scope and spawn a task that holds it alive (WASM, no Send).
  #[cfg(feature = "wasm")]
  pub fn spawn_and_hold<F, Fut>(self, name: &str, f: F)
  where
    F: FnOnce(CancellationToken) -> Fut,
    Fut: Future<Output = ()> + 'static,
  {
    let (id, task_token, span) = self.register_task(name);
    let fut = f(task_token.clone());
    async_manager::spawn(
      async move {
        let _hold = self;
        finish_task(fut, id, task_token).await;
      },
      span,
    );
  }

  fn register_task(&self, name: &str) -> (TaskId, CancellationToken, tracing::Span) {
    let path = format!("{}/{}", self.path, name);
    let id = registry().register(path.clone(), false);
    let task_token = self.token.child_token();
    // Span names must be const in tracing; the dynamic path goes in a field.
    let span = tracing::span!(tracing::Level::INFO, "buttplug_task", task.path = %path);
    (id, task_token, span)
  }
}

/// Deregisters a task from the global registry on drop. Created BEFORE the
/// task future is awaited so deregistration happens even if the future panics:
/// a panicking task unwinds through this guard, so the registry entry is removed
/// rather than leaked (which would hang `wait_empty_under` on that subtree
/// forever). The outcome is derived at drop time from whether we are unwinding
/// from a panic and whether the task observed cancellation.
///
/// `token` is `None` for detached tasks, which have no cancellation concept:
/// their outcome is `Panicked` on panic, else `Completed`.
pub(super) struct DeregisterGuard {
  id: TaskId,
  token: Option<CancellationToken>,
}

impl DeregisterGuard {
  pub(super) fn new(id: TaskId, token: Option<CancellationToken>) -> Self {
    Self { id, token }
  }
}

impl Drop for DeregisterGuard {
  fn drop(&mut self) {
    let outcome = if std::thread::panicking() {
      TaskOutcome::Panicked
    } else if self.token.as_ref().is_some_and(|t| t.is_cancelled()) {
      TaskOutcome::Cancelled
    } else {
      TaskOutcome::Completed
    };
    registry().deregister(self.id, outcome);
  }
}

async fn finish_task(fut: impl Future<Output = ()>, id: TaskId, token: CancellationToken) {
  let _guard = DeregisterGuard::new(id, Some(token));
  fut.await;
}

impl Drop for TaskScope {
  fn drop(&mut self) {
    self.token.cancel();
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use crate::util::task::registry;
  use std::time::Duration;

  #[test]
  fn test_root_path_unique() {
    let a = TaskScope::root("testroot");
    let b = TaskScope::root("testroot");
    assert_ne!(a.path(), b.path());
    assert!(a.path().starts_with("testroot-"));
  }

  #[test]
  fn test_child_path() {
    let root = TaskScope::root("testroot");
    let child = root.child("devices");
    assert_eq!(child.path(), format!("{}/devices", root.path()));
  }

  #[tokio::test]
  async fn test_spawn_registers_and_completes() {
    let root = TaskScope::root("spawntest");
    let path = root.path().to_owned();
    let (tx, rx) = tokio::sync::oneshot::channel();
    root.spawn("worker", |_token| async move {
      let _ = tx.send(());
    });
    rx.await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), registry().wait_empty_under(&path))
      .await
      .expect("task did not deregister after completion");
  }

  #[tokio::test]
  async fn test_cancel_propagates_to_children() {
    let root = TaskScope::root("canceltest");
    let path = root.path().to_owned();
    let child = root.child("inner");
    child.spawn("worker", |token| async move {
      token.cancelled().await;
    });
    root.cancel();
    tokio::time::timeout(Duration::from_secs(1), registry().wait_empty_under(&path))
      .await
      .expect("cancel did not propagate to child scope task");
  }

  #[tokio::test]
  async fn test_drop_cancels() {
    let root = TaskScope::root("droptest");
    let path = root.path().to_owned();
    root.spawn("worker", |token| async move {
      token.cancelled().await;
    });
    drop(root);
    tokio::time::timeout(Duration::from_secs(1), registry().wait_empty_under(&path))
      .await
      .expect("drop did not cancel task");
  }

  #[tokio::test]
  async fn test_shutdown_awaits_subtree() {
    let root = TaskScope::root("shutdowntest");
    root.spawn("worker", |token| async move {
      token.cancelled().await;
      // Simulate cleanup work after observing cancellation.
      tokio::time::sleep(Duration::from_millis(20)).await;
    });
    tokio::time::timeout(Duration::from_secs(1), root.shutdown())
      .await
      .expect("shutdown did not resolve");
  }

  #[tokio::test]
  async fn test_panicking_task_deregisters() {
    // A scoped task that panics must still deregister (via the drop guard),
    // otherwise wait_empty_under on its root would hang forever. tokio catches
    // the panic at the task boundary, so this test itself does not fail from the
    // spawned panic. Without the guard, this wait would time out.
    let root = TaskScope::root("panictest");
    let path = root.path().to_owned();
    root.spawn("panicker", |_token| async move {
      panic!("intentional panic for deregistration test");
    });
    tokio::time::timeout(Duration::from_secs(1), registry().wait_empty_under(&path))
      .await
      .expect("panicking task did not deregister — registry entry leaked");
  }

  #[tokio::test]
  async fn test_spawn_and_hold_keeps_scope_alive() {
    use registry::TaskEvent;

    let root = TaskScope::root("holdtest");
    let path = root.path().to_owned();
    // Subscribe BEFORE spawning so we observe both the Started and Ended events
    // for the held task and can assert how it actually finished.
    let mut events = registry().event_stream();
    let sub_scope = root.child("subscription");
    let (tx, rx) = tokio::sync::oneshot::channel();
    // Consuming spawn: the scope moves INTO the task and must not cancel it.
    sub_scope.spawn_and_hold("worker", |_token| async move {
      tokio::time::sleep(Duration::from_millis(20)).await;
      let _ = tx.send(());
    });
    // If drop-cancel fired early this would hang (task cancelled before send).
    tokio::time::timeout(Duration::from_secs(1), rx)
      .await
      .expect("spawn_and_hold task was cancelled early")
      .unwrap();
    tokio::time::timeout(Duration::from_secs(1), registry().wait_empty_under(&path))
      .await
      .expect("task did not deregister");

    // The held task ran to its natural end, so its reported outcome MUST be
    // Completed — not Cancelled. (If spawn_and_hold had wired drop-cancel to the
    // held task, it would have been cancelled mid-sleep and reported Cancelled.)
    // Drain the event stream looking for this task's Ended event. The path is
    // exact ("<root>/subscription/worker") so we don't match unrelated tasks
    // from other tests sharing the global registry.
    let task_path = format!("{path}/subscription/worker");
    let outcome = tokio::time::timeout(Duration::from_secs(1), async {
      loop {
        match events.recv().await {
          Ok(TaskEvent::Ended { path, outcome, .. }) if path == task_path => return outcome,
          Ok(_) => continue,
          // The registry's broadcast channel is process-global; heavy parallel
          // test load can evict buffered events (Lagged). Keep draining — our
          // own Ended event fires ~20ms after subscribing, so under normal load
          // it arrives well before any eviction window matters.
          Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
          Err(tokio::sync::broadcast::error::RecvError::Closed) => {
            panic!("event stream closed before held task's Ended event")
          }
        }
      }
    })
    .await
    .expect("did not observe Ended event for held task");
    assert_eq!(
      outcome,
      TaskOutcome::Completed,
      "normally-finishing held task should report Completed, got {outcome:?}"
    );
  }
}
