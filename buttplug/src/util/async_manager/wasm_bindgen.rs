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

pub fn spawn<Fut>(future: Fut) -> Result<(), SpawnError>
where
  Fut: Future<Output = ()> + Send + 'static,
{
  WasmBindgenAsyncManager::default().spawn(future)
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  WasmBindgenAsyncManager::default().spawn_with_handle(future)
}

pub fn block_on<F>(future: F) -> <F as Future>::Output
where
  F: Future,
{
  unimplemented!("Can't block in wasm!")
}
