// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{TaskCompletion, TaskCompletionResult};
use futures::{channel::oneshot, future::BoxFuture, task::LocalFutureObj};
use std::time::Duration;
use tracing::{Instrument, Span};

#[derive(Default, Debug)]
pub struct WasmBindgenAsyncManager {}

impl super::AsyncManager for WasmBindgenAsyncManager {
  fn spawn(
    &self,
    future: LocalFutureObj<'static, TaskCompletionResult>,
    span: Span,
  ) -> TaskCompletion {
    let (sender, receiver) = oneshot::channel();
    wasm_bindgen_futures::spawn_local(
      async move {
        let _ = sender.send(future.await);
      }
      .instrument(span),
    );
    Box::pin(async move { receiver.await.unwrap_or(TaskCompletionResult::Cancelled) })
  }

  fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(wasmtimer::tokio::sleep(duration))
  }
}
