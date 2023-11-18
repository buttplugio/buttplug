// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use futures::{
  future::{Future, RemoteHandle},
  task::{FutureObj, Spawn, SpawnError, SpawnExt},
};
use tokio;

#[derive(Default)]
pub struct TokioAsyncManager {}

impl Spawn for TokioAsyncManager {
  fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    tokio::spawn(future);
    Ok(())
  }
}

pub fn spawn<Fut>(future: Fut)
where
  Fut: Future<Output = ()> + Send + 'static,
{
  TokioAsyncManager::default()
    .spawn(future)
    .expect("Infallible, only returns result to match trait")
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  TokioAsyncManager::default().spawn_with_handle(future)
}

pub fn block_on<F>(f: F) -> <F as Future>::Output
where
  F: Future,
{
  let handle = tokio::runtime::Handle::current();
  let _ = handle.enter();
  // Execute the future, blocking the current thread until completion
  futures::executor::block_on(async move { f.await })
}
