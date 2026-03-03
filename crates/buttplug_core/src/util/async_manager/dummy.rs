// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use futures::task::FutureObj;

#[derive(Default, Debug)]
pub struct DummyAsyncManager {}

#[async_trait]
impl super::AsyncManager for DummyAsyncManager {
  fn spawn(&self, _future: FutureObj<'static, ()>) {
    unimplemented!(
      "No async runtime available. Please set a global async manager using set_global_async_manager or enable tokio-runtime or wasm feature"
    );
  }

  async fn sleep(&self, _duration: std::time::Duration) {
    unimplemented!(
      "No async runtime available. Please set a global async manager using set_global_async_manager or enable tokio-runtime or wasm feature"
    );
  }

  async fn sleep_until(&self, _deadline: std::time::Instant) {
    unimplemented!(
      "No async runtime available. Please set a global async manager using set_global_async_manager or enable tokio-runtime or wasm feature"
    );
  }
}
