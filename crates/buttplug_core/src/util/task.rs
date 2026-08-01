// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::util::async_manager::{self, TaskCompletion, TaskCompletionResult};
use futures::{
  channel::oneshot,
  future::{AbortHandle, Abortable, BoxFuture, FutureExt, Shared},
};
use std::{
  future::Future,
  sync::{Arc, Mutex},
};
use tracing::Span;

/// Build the [`Span`] identifying a task passed to [`TaskGroup::spawn`].
///
/// The name must be a literal: a span's name is baked into its `'static` callsite
/// metadata, so it cannot come from a runtime value.
#[macro_export]
macro_rules! task_span {
  ($name:expr) => {
    tracing::span!(tracing::Level::INFO, $name)
  };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskGroupClosed;

struct OwnedTask {
  abort_handle: AbortHandle,
  completion: TaskCompletion,
}

type ShutdownCompletion = Shared<BoxFuture<'static, Vec<TaskCompletionResult>>>;

#[derive(Default)]
struct TaskGroupState {
  closed: bool,
  tasks: Vec<OwnedTask>,
  shutdown: Option<ShutdownCompletion>,
}

#[derive(Default)]
struct TaskGroupInner {
  state: Mutex<TaskGroupState>,
}

impl Drop for TaskGroupInner {
  fn drop(&mut self) {
    let state = self
      .state
      .get_mut()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.closed = true;
    for task in &state.tasks {
      task.abort_handle.abort();
    }
  }
}

#[derive(Clone, Default)]
pub struct TaskGroup {
  inner: Arc<TaskGroupInner>,
}

impl TaskGroup {
  pub fn new() -> Self {
    Self::default()
  }

  fn reserve(
    &self,
  ) -> Result<
    (
      futures::future::AbortRegistration,
      oneshot::Sender<TaskCompletion>,
    ),
    TaskGroupClosed,
  > {
    let mut state = self
      .inner
      .state
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    if state.closed {
      return Err(TaskGroupClosed);
    }

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let (completion_sender, completion_receiver) = oneshot::channel::<TaskCompletion>();
    let completion = async move {
      match completion_receiver.await {
        Ok(completion) => completion.await,
        Err(_) => TaskCompletionResult::RuntimeAborted,
      }
    }
    .boxed();
    state.tasks.push(OwnedTask {
      abort_handle,
      completion,
    });
    Ok((abort_registration, completion_sender))
  }

  /// Spawn a task into this group, identified by `span`.
  ///
  /// Build `span` at the call site with a literal name, via the [`task_span!`] macro or
  /// `tracing::span!` directly. A span's name lives in its callsite metadata, which
  /// [`AsyncManager`][async_manager::AsyncManager] implementations can read; a name passed
  /// as a span *field* is only visible to tracing subscribers, so runtimes that allocate
  /// per-task resources by name could not see it.
  ///
  /// Note that `Span::metadata()` returns `Some` only while the span is *enabled*: the
  /// active subscriber's filter must accept the INFO-level callsite. Runtimes that read
  /// task names from span metadata must install a subscriber that enables these spans
  /// before spawning, or every task arrives with `metadata() == None`.
  #[cfg(not(feature = "wasm"))]
  pub fn spawn<F, Fut>(&self, span: Span, task: F) -> Result<(), TaskGroupClosed>
  where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
  {
    let (abort_registration, completion_sender) = self.reserve()?;
    let future = async move {
      match Abortable::new(task(), abort_registration).await {
        Ok(()) => TaskCompletionResult::Completed,
        Err(_) => TaskCompletionResult::Cancelled,
      }
    };
    let completion = async_manager::spawn_with_result(future, span);
    let _ = completion_sender.send(completion);
    Ok(())
  }

  /// Spawn a task into this group, identified by `span`.
  ///
  /// See the non-WASM variant for why this takes a [`Span`] rather than a name.
  #[cfg(feature = "wasm")]
  pub fn spawn<F, Fut>(&self, span: Span, task: F) -> Result<(), TaskGroupClosed>
  where
    F: FnOnce() -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
  {
    let (abort_registration, completion_sender) = self.reserve()?;
    let future = async move {
      match Abortable::new(task(), abort_registration).await {
        Ok(()) => TaskCompletionResult::Completed,
        Err(_) => TaskCompletionResult::Cancelled,
      }
    };
    let completion = async_manager::spawn_with_result(future, span);
    let _ = completion_sender.send(completion);
    Ok(())
  }

  pub fn cancel(&self) {
    let mut state = self
      .inner
      .state
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    state.closed = true;
    for task in &state.tasks {
      task.abort_handle.abort();
    }
  }

  pub async fn shutdown(&self) -> Vec<TaskCompletionResult> {
    let shutdown = {
      let mut state = self
        .inner
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
      if let Some(shutdown) = &state.shutdown {
        shutdown.clone()
      } else {
        state.closed = true;
        let tasks = std::mem::take(&mut state.tasks);
        for task in &tasks {
          task.abort_handle.abort();
        }
        let shutdown = async move {
          futures::future::join_all(tasks.into_iter().map(|task| task.completion)).await
        }
        .boxed()
        .shared();
        state.shutdown = Some(shutdown.clone());
        shutdown
      }
    };

    shutdown.await
  }
}

#[cfg(all(test, not(feature = "wasm")))]
mod tests {
  use super::*;
  use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
  use tokio::sync::oneshot;

  /// `AsyncManager` implementations for runtimes that allocate per-task resources (stack
  /// size on FreeRTOS, for instance) can only identify a task by its callsite metadata
  /// name. A name passed as a span *field* is invisible to them, so every task would
  /// arrive indistinguishable and get the same fallback allocation.
  #[test]
  fn task_span_carries_name_in_callsite_metadata() {
    tracing::subscriber::with_default(tracing_subscriber::registry(), || {
      let span = crate::task_span!("DeviceTask");
      assert_eq!(
        span.metadata().map(|metadata| metadata.name()),
        Some("DeviceTask")
      );
    });
  }

  #[tokio::test]
  async fn spawned_task_is_joined_by_shutdown() {
    let group = TaskGroup::new();
    let (started_sender, started_receiver) = oneshot::channel();
    let (dropped_sender, dropped_receiver) = oneshot::channel();
    group
      .spawn(crate::task_span!("joined"), || async move {
        let _guard = DropSignal(Some(dropped_sender));
        let _ = started_sender.send(());
        futures::future::pending::<()>().await;
      })
      .unwrap();
    started_receiver.await.unwrap();

    assert_eq!(
      group.shutdown().await,
      vec![TaskCompletionResult::Cancelled]
    );
    dropped_receiver.await.unwrap();
  }

  #[tokio::test]
  async fn spawn_rejected_after_shutdown_begins() {
    let group = TaskGroup::new();
    group.cancel();
    let invoked = Arc::new(AtomicBool::new(false));
    let invoked_for_task = invoked.clone();

    assert_eq!(
      group.spawn(crate::task_span!("rejected"), move || {
        invoked_for_task.store(true, Ordering::SeqCst);
        async {}
      }),
      Err(TaskGroupClosed)
    );
    assert!(!invoked.load(Ordering::SeqCst));
  }

  #[test]
  fn concurrent_spawn_is_rejected_or_joined() {
    let group = TaskGroup::new();
    let invoked = Arc::new(AtomicBool::new(false));
    let mut state = group.inner.state.lock().unwrap();

    let spawn_group = group.clone();
    let invoked_for_task = invoked.clone();
    let spawn = std::thread::spawn(move || {
      spawn_group.spawn(crate::task_span!("racing spawn"), move || {
        invoked_for_task.store(true, Ordering::SeqCst);
        async {}
      })
    });
    state.closed = true;
    drop(state);

    assert_eq!(spawn.join().unwrap(), Err(TaskGroupClosed));
    assert!(!invoked.load(Ordering::SeqCst));
  }

  #[tokio::test]
  async fn task_panic_does_not_hang_shutdown() {
    let group = TaskGroup::new();
    let (started_sender, started_receiver) = oneshot::channel();
    group
      .spawn(crate::task_span!("panic"), || async move {
        let _ = started_sender.send(());
        panic!("expected panic");
      })
      .unwrap();
    started_receiver.await.unwrap();

    assert_eq!(group.shutdown().await, vec![TaskCompletionResult::Panicked]);
  }

  #[tokio::test]
  async fn concurrent_shutdown_callers_share_completion() {
    let group = TaskGroup::new();
    let (started_sender, started_receiver) = oneshot::channel();
    group
      .spawn(crate::task_span!("concurrent shutdown"), || async move {
        let _ = started_sender.send(());
        futures::future::pending::<()>().await;
      })
      .unwrap();
    started_receiver.await.unwrap();

    let first = group.shutdown();
    let second = group.shutdown();
    let (first, second) = futures::future::join(first, second).await;
    assert_eq!(first, vec![TaskCompletionResult::Cancelled]);
    assert_eq!(second, first);
  }

  #[tokio::test]
  async fn sequential_shutdown_is_idempotent() {
    let group = TaskGroup::new();
    let (started_sender, started_receiver) = oneshot::channel();
    group
      .spawn(crate::task_span!("sequential shutdown"), || async move {
        let _ = started_sender.send(());
        futures::future::pending::<()>().await;
      })
      .unwrap();
    started_receiver.await.unwrap();

    let first = group.shutdown().await;
    let second = group.shutdown().await;
    assert_eq!(first, vec![TaskCompletionResult::Cancelled]);
    assert_eq!(second, first);
  }

  #[tokio::test]
  async fn drop_requests_cancellation() {
    let (started_sender, started_receiver) = oneshot::channel();
    let (dropped_sender, dropped_receiver) = oneshot::channel();
    {
      let group = TaskGroup::new();
      group
        .spawn(crate::task_span!("drop"), || async move {
          let _guard = DropSignal(Some(dropped_sender));
          let _ = started_sender.send(());
          futures::future::pending::<()>().await;
        })
        .unwrap();
      started_receiver.await.unwrap();
    }

    dropped_receiver.await.unwrap();
  }

  #[tokio::test]
  async fn concurrent_final_clone_drops_request_cancellation() {
    let (started_sender, started_receiver) = oneshot::channel();
    let (dropped_sender, dropped_receiver) = oneshot::channel();
    let group = TaskGroup::new();
    group
      .spawn(crate::task_span!("concurrent drops"), || async move {
        let _guard = DropSignal(Some(dropped_sender));
        let _ = started_sender.send(());
        futures::future::pending::<()>().await;
      })
      .unwrap();
    started_receiver.await.unwrap();

    let other = group.clone();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let first_barrier = barrier.clone();
    let first = std::thread::spawn(move || {
      first_barrier.wait();
      drop(group);
    });
    let second_barrier = barrier.clone();
    let second = std::thread::spawn(move || {
      second_barrier.wait();
      drop(other);
    });
    barrier.wait();
    first.join().unwrap();
    second.join().unwrap();

    dropped_receiver.await.unwrap();
  }

  #[tokio::test]
  async fn duplicate_names_remain_independent() {
    let group = TaskGroup::new();
    let completed = Arc::new(AtomicUsize::new(0));
    let mut started = Vec::new();
    for _ in 0..2 {
      let completed = completed.clone();
      let (started_sender, started_receiver) = oneshot::channel();
      started.push(started_receiver);
      group
        .spawn(crate::task_span!("duplicate"), move || async move {
          completed.fetch_add(1, Ordering::SeqCst);
          let _ = started_sender.send(());
        })
        .unwrap();
    }
    for receiver in started {
      receiver.await.unwrap();
    }

    let results = group.shutdown().await;
    assert_eq!(results.len(), 2);
    assert_eq!(completed.load(Ordering::SeqCst), 2);
  }

  #[test]
  fn runtime_drop_with_live_tasks_does_not_poison_next_runtime() {
    let first_runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    let completion = {
      let _guard = first_runtime.enter();
      async_manager::spawn(
        futures::future::pending::<()>(),
        tracing::span!(tracing::Level::INFO, "runtime drop test"),
      )
    };
    drop(first_runtime);

    let second_runtime = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    assert_eq!(
      second_runtime.block_on(completion),
      TaskCompletionResult::RuntimeAborted
    );
    let next_completion = {
      let _guard = second_runtime.enter();
      async_manager::spawn(
        async {},
        tracing::span!(tracing::Level::INFO, "replacement runtime test"),
      )
    };
    assert_eq!(
      second_runtime.block_on(next_completion),
      TaskCompletionResult::Completed
    );
  }

  #[test]
  fn repeated_runtime_shutdown_completes_owned_tasks() {
    for _ in 0..3 {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
      let group = TaskGroup::new();
      let _guard = runtime.enter();
      group
        .spawn(crate::task_span!("runtime cycle"), || async {
          futures::future::pending::<()>().await;
        })
        .unwrap();
      assert_eq!(
        runtime.block_on(group.shutdown()),
        vec![TaskCompletionResult::Cancelled]
      );
    }
  }

  struct DropSignal(Option<oneshot::Sender<()>>);

  impl Drop for DropSignal {
    fn drop(&mut self) {
      if let Some(sender) = self.0.take() {
        let _ = sender.send(());
      }
    }
  }
}
