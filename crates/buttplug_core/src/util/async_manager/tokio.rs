// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use futures::{future::BoxFuture, task::FutureObj};
use std::time::Duration;
use tracing::{Instrument, Span};

#[derive(Default, Debug)]
pub struct TokioAsyncManager {}

impl super::AsyncManager for TokioAsyncManager {
  fn spawn(&self, future: FutureObj<'static, ()>, span: Span) {
    tokio::task::spawn(future.instrument(span));
  }

  fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
    Box::pin(tokio::time::sleep(duration))
  }
}
