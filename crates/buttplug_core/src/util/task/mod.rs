// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Task Scope and Task Registry: ownership and introspection for spawned tasks.
//!
//! Every task is spawned through a [TaskScope], which links it into an
//! ownership tree, derives its hierarchical path, registers it in the global
//! [TaskRegistry], and hands it a cooperative [CancellationToken]. Dropping a
//! scope cancels its subtree.

mod registry;
mod scope;

pub use registry::{TaskEvent, TaskId, TaskInfo, TaskOutcome, TaskRegistry, registry};
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
      fut.await;
      registry().deregister(id, TaskOutcome::Completed);
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
      fut.await;
      registry().deregister(id, TaskOutcome::Completed);
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
