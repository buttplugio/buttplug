// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::OnceLock;

use async_trait::async_trait;
use futures::task::FutureObj;
use tracing::Span;

cfg_if::cfg_if! {
  if #[cfg(feature = "wasm")] {
    mod wasm;
    use self::wasm::{WasmAsyncManager as DefaultAsyncManager};
  } else if #[cfg(feature = "tokio-runtime")] {
    mod tokio;
    use self::tokio::{TokioAsyncManager as DefaultAsyncManager};
  } else {
    mod dummy;
    use dummy::{DummyAsyncManager as DefaultAsyncManager};
  }
}

static GLOBAL_ASYNC_MANAGER: OnceLock<Box<dyn AsyncManager + Send + Sync>> = OnceLock::new();

pub fn set_global_async_manager(manager: Box<dyn AsyncManager + Send + Sync>) {
  log::info!("Setting global async manager to {:?}", manager);
  GLOBAL_ASYNC_MANAGER
    .set(manager)
    .expect("Global async manager can only be set once.");
}

fn get_global_async_manager() -> &'static Box<dyn AsyncManager + Send + Sync> {
  GLOBAL_ASYNC_MANAGER.get_or_init(|| {
    let default_manager = DefaultAsyncManager::default();
    log::info!(
      "No global async manager set, using {:?} according to feature flag.",
      default_manager
    );
    Box::new(default_manager)
  })
}

/// The `AsyncManager` is a trait that abstracts over the async runtime used by Buttplug.
/// It is similar to [futures::task::Spawn] but also includes sleep functions since they also depend on the used async runtime.
/// It also forces instumentation of tracing spans for all spawned tasks.
/// This usually does not need to be used in user code, but is public to allow users to implement their own async runtimes if needed.
#[async_trait]
pub trait AsyncManager: std::fmt::Debug + Send + Sync {
  /// Spawns a future onto the async runtime. The future must be `Send` and `'static` since it may be spawned onto a different thread.
  /// The span parameter should be used to instrument the future with a tracing span.
  fn spawn(&self, future: FutureObj<'static, ()>, span: Span);
  async fn sleep(&self, duration: std::time::Duration);
  async fn sleep_until(&self, deadline: std::time::Instant);
}

/// Spawns a future onto the global async manager.
pub fn spawn<F>(future: F, span: Span)
where
  F: Future<Output = ()> + Send + 'static,
{
  let async_manager = get_global_async_manager();

  async_manager.spawn(Box::new(future).into(), span);
}

/// Sleeps for the specified duration using the global async manager.
pub async fn sleep(duration: std::time::Duration) {
  let async_manager = get_global_async_manager();

  async_manager.sleep(duration).await;
}

/// Sleeps until the specified deadline using the global async manager.
pub async fn sleep_until(deadline: std::time::Instant) {
  let async_manager = get_global_async_manager();

  async_manager.sleep_until(deadline).await;
}
