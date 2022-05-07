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

use wasm_bindgen_futures::spawn_local;

#[derive(Default)]
pub struct WasmBindgenAsyncManager {}

impl Spawn for WasmBindgenAsyncManager {
  fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    spawn_local(future);
    Ok(())
  }
}

pub fn spawn<Fut>(future: Fut)
where
  Fut: Future<Output = ()> + 'static,
{
  spawn_local(future);
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  WasmBindgenAsyncManager::default().spawn_with_handle(future)
}

pub fn block_on<F>(_: F) -> <F as Future>::Output
where
  F: Future,
{
  unimplemented!("Can't block in wasm!")
}
