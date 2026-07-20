// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Task Scope and Task Registry: ownership and introspection for spawned tasks.
//!
//! A task spawned through a [TaskScope] is linked into an ownership tree, given
//! a hierarchical path, registered in the global [TaskRegistry], and handed a
//! cooperative [CancellationToken]; dropping a scope cancels its subtree. The
//! `buttplug_server` crate is fully migrated onto scope-owned tasks; the
//! hardware-manager and client crates still use the bare `spawn!` macro and
//! migrate onto scopes in a follow-up.

mod registry;
mod scope;

pub use registry::{
  TaskEvent,
  TaskId,
  TaskInfo,
  TaskOutcome,
  TaskRegistry,
  TaskSpawnError,
  registry,
};
pub use scope::TaskScope;

use crate::util::async_manager;
use std::future::Future;

/// Spawn a task with no owning scope. Registered in the Task Registry under
/// "detached/{name}" so it still shows up in snapshots, but nothing can cancel
/// it. RARE — prefer [TaskScope::spawn]. Valid uses: one-shot notifications
/// where the spawner is being destroyed.
#[cfg(not(feature = "wasm"))]
pub fn spawn_detached<Fut>(name: &str, fut: Fut)
where
  Fut: Future<Output = ()> + Send + 'static,
{
  let path = format!("detached/{name}");
  let id = registry().register(path.clone(), true);
  let span = tracing::span!(tracing::Level::INFO, "buttplug_task", task.path = %path);
  async_manager::spawn(
    async move {
      // Deregister via a drop guard so a panicking detached task still leaves
      // the registry (outcome Panicked) instead of leaking its entry forever.
      // Detached tasks have no cancellation token, so the outcome is Panicked on
      // panic, else Completed.
      let _guard = scope::DeregisterGuard::new(id, None);
      fut.await;
    },
    span,
  );
}

/// Spawn a task with no owning scope (WASM, no Send required). See the
/// non-WASM variant for semantics.
#[cfg(feature = "wasm")]
pub fn spawn_detached<Fut>(name: &str, fut: Fut)
where
  Fut: Future<Output = ()> + 'static,
{
  let path = format!("detached/{name}");
  let id = registry().register(path.clone(), true);
  let span = tracing::span!(tracing::Level::INFO, "buttplug_task", task.path = %path);
  async_manager::spawn(
    async move {
      // Deregister via a drop guard so a panicking detached task still leaves
      // the registry (outcome Panicked) instead of leaking its entry forever.
      // Detached tasks have no cancellation token, so the outcome is Panicked on
      // panic, else Completed.
      let _guard = scope::DeregisterGuard::new(id, None);
      fut.await;
    },
    span,
  );
}

#[cfg(test)]
mod test {
  use super::*;
  use std::time::Duration;

  #[tokio::test]
  async fn test_spawn_detached_registers() {
    let (tx, rx) = tokio::sync::oneshot::channel();
    spawn_detached("test-notify", async move {
      let _ = tx.send(());
    });
    rx.await.unwrap();
    tokio::time::timeout(
      Duration::from_secs(1),
      registry().wait_empty_under("detached/test-notify"),
    )
    .await
    .expect("detached task did not deregister");
  }
}
