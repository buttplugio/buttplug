use async_std::task;
use futures::{
  future::{Future, RemoteHandle},
  task::{FutureObj, Spawn, SpawnError, SpawnExt},
};

#[derive(Default)]
pub struct AsyncStdAsyncManager {}

#[cfg(target_arch = "wasm32")]
impl Spawn for AsyncStdAsyncManager {
  fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    task::spawn_local(future);
    Ok(())
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Spawn for AsyncStdAsyncManager {
  fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    task::spawn(future);
    Ok(())
  }
}

pub fn spawn<Fut>(future: Fut) -> Result<(), SpawnError>
where
  Fut: Future<Output = ()> + Send + 'static,
{
  AsyncStdAsyncManager::default().spawn(future)
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  AsyncStdAsyncManager::default().spawn_with_handle(future)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn block_on<F>(f: F) -> <F as Future>::Output
where
  F: Future,
{
  task::block_on(f)
}

#[cfg(target_arch = "wasm32")]
pub fn block_on<F>(f: F) -> <F as Future>::Output
where
  F: Future,
{
  unimplemented!("No block_on in wasm")
}
