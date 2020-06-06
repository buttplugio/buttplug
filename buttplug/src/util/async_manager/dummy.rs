use futures::{
  future::{Future, RemoteHandle},
  task::{FutureObj, Spawn, SpawnError},
};

#[derive(Default)]
pub struct DummyAsyncManager {}

impl Spawn for DummyAsyncManager {
  fn spawn_obj(&self, _: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    unimplemented!("Dummy executor can't actually spawn!")
  }
}

pub fn spawn<Fut>(_: Fut) -> Result<(), SpawnError>
where
  Fut: Future<Output = ()> + Send + 'static,
{
  unimplemented!("Dummy executor can't actually spawn!")
}

pub fn spawn_with_handle<Fut>(_: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  unimplemented!("Dummy executor can't actually spawn!")
}

pub fn block_on<F>(_: F) -> <F as Future>::Output
where
  F: Future,
{
  unimplemented!("Dummy executor can't actually spawn!")
}
