// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use futures::future::BoxFuture;
#[cfg(not(feature = "wasm"))]
use futures::task::FutureObj;
#[cfg(feature = "wasm")]
use futures::task::LocalFutureObj;
use std::{future::Future, sync::OnceLock, time::Duration};
use tracing::Span;

/// The terminal state of a spawned task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskCompletionResult {
  Completed,
  Panicked,
  Cancelled,
  RuntimeAborted,
}

/// Runtime-neutral handle used to await a spawned task's completion.
pub type TaskCompletion = BoxFuture<'static, TaskCompletionResult>;

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(all(feature = "tokio-runtime", not(feature = "wasm")))]
mod tokio;

static GLOBAL_ASYNC_MANAGER: OnceLock<Box<dyn AsyncManager>> = OnceLock::new();

/// Set a custom global async manager.
///
/// Call this once at startup to plug in a non-default async runtime. If not
/// called, the default manager for the enabled feature flag is used.
///
/// # Panics
/// Panics if called more than once.
pub fn set_global_async_manager(manager: Box<dyn AsyncManager>) {
  GLOBAL_ASYNC_MANAGER
    .set(manager)
    .expect("Global async manager can only be set once.");
}

/// Get the default async manager based on enabled feature flags.
fn get_default_async_manager() -> Box<dyn AsyncManager> {
  cfg_if::cfg_if! {
    if #[cfg(feature = "wasm")] {
      return Box::new(wasm::WasmBindgenAsyncManager::default());
    } else if #[cfg(feature = "tokio-runtime")] {
      Box::new(tokio::TokioAsyncManager::default())
    } else {
      unimplemented!(
        "No async runtime configured. Enable the `tokio-runtime` or `wasm` feature, \
          or call `set_global_async_manager` before performing async operations."
      );
    }
  }
}

fn get_global_async_manager() -> &'static dyn AsyncManager {
  GLOBAL_ASYNC_MANAGER
    .get_or_init(|| get_default_async_manager())
    .as_ref()
}

/// Trait for async runtime abstraction in Buttplug.
///
/// Implement this trait to plug in a custom async runtime, then pass it to
/// [`set_global_async_manager`] before any async operations are performed.
///
/// Built-in implementations are provided for Tokio (via `tokio-runtime` feature)
/// and WASM (via `wasm` feature). For other runtimes (e.g. Embassy, esp-idf),
/// implement this trait and call [`set_global_async_manager`] at startup.
///
/// Implementations must treat spawning as a transactional operation. They must return without
/// synchronously polling the submitted future. Once the runtime accepts the future, the method
/// must return exactly one completion handle and must not unwind. These requirements let task
/// owners account for and join every accepted task without depending on a specific runtime.
pub trait AsyncManager: std::fmt::Debug + Send + Sync {
  /// Spawn a task on the async runtime and return its runtime-neutral completion handle.
  ///
  /// The `span` should be used to instrument the task with tracing context.
  #[cfg(not(feature = "wasm"))]
  fn spawn(&self, future: FutureObj<'static, TaskCompletionResult>, span: Span) -> TaskCompletion;

  /// Spawn a task on the async runtime and return its runtime-neutral completion handle.
  ///
  /// WASM runtimes cannot report task panics, so a dropped completion sender is reported as
  /// [`TaskCompletionResult::Cancelled`].
  #[cfg(feature = "wasm")]
  fn spawn(
    &self,
    future: LocalFutureObj<'static, TaskCompletionResult>,
    span: Span,
  ) -> TaskCompletion;

  /// Sleep for the given duration.
  fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()>;
}

/// Spawn a fire-and-forget task on the global async manager.
///
/// Prefer the [`spawn!`][crate::spawn] macro for ergonomic use with a task name.
#[cfg(not(feature = "wasm"))]
pub fn spawn<F>(future: F, span: Span) -> TaskCompletion
where
  F: Future<Output = ()> + Send + 'static,
{
  spawn_with_result(
    async move {
      future.await;
      TaskCompletionResult::Completed
    },
    span,
  )
}

#[cfg(not(feature = "wasm"))]
pub(crate) fn spawn_with_result<F>(future: F, span: Span) -> TaskCompletion
where
  F: Future<Output = TaskCompletionResult> + Send + 'static,
{
  get_global_async_manager().spawn(FutureObj::new(Box::new(future)), span)
}

/// Spawn a fire-and-forget task on the global async manager (WASM, no Send required).
///
/// Prefer the [`spawn!`][crate::spawn] macro for ergonomic use with a task name.
#[cfg(feature = "wasm")]
pub fn spawn<F>(future: F, span: Span) -> TaskCompletion
where
  F: Future<Output = ()> + 'static,
{
  spawn_with_result(
    async move {
      future.await;
      TaskCompletionResult::Completed
    },
    span,
  )
}

#[cfg(feature = "wasm")]
pub(crate) fn spawn_with_result<F>(future: F, span: Span) -> TaskCompletion
where
  F: Future<Output = TaskCompletionResult> + 'static,
{
  get_global_async_manager().spawn(LocalFutureObj::new(Box::new(future)), span)
}

/// Sleep for the given duration using the global async manager.
pub async fn sleep(duration: Duration) {
  get_global_async_manager().sleep(duration).await;
}

/// Spawn a fire-and-forget task on the global async manager.
/// Always prefer to add a name to the task for better tracing context.
#[macro_export]
macro_rules! spawn {
  ($future:expr) => {{
    let _ = $crate::util::async_manager::spawn(
      $future,
      tracing::span!(tracing::Level::INFO, "Buttplug Async Task"),
    );
  }};
  ($name:expr, $future:expr) => {{
    let _ =
      $crate::util::async_manager::spawn($future, tracing::span!(tracing::Level::INFO, $name));
  }};
}
