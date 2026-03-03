// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use futures::task::FutureObj;

use wasm_bindgen_futures::spawn_local;
use wasm_timer::tokio::{sleep, sleep_until};

#[derive(Default, Debug)]
pub struct WasmAsyncManager {}

#[async_trait]
impl super::AsyncManager for WasmAsyncManager {
  fn spawn(&self, future: FutureObj<'static, ()>) {
    spawn_local(future);
  }

  async fn sleep(&self, duration: std::time::Duration) {
    sleep(duration).await;
  }

  async fn sleep_until(&self, deadline: std::time::Instant) {
    sleep_until(deadline).await;
  }
}
