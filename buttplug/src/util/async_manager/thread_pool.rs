use futures::{
  executor::{block_on as block_on_executor, ThreadPool},
  future::{Future, RemoteHandle},
  task::{FutureObj, Spawn, SpawnError, SpawnExt},
};

lazy_static! {
  static ref THREAD_POOL: ThreadPool = ThreadPool::new().unwrap();
}

#[derive(Default)]
pub struct ThreadPoolAsyncManager {}

impl Spawn for ThreadPoolAsyncManager {
  fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
    THREAD_POOL.spawn_obj(future)
  }
}

pub fn spawn<Fut>(future: Fut) -> Result<(), SpawnError>
where
  Fut: Future<Output = ()> + Send + 'static,
{
  ThreadPoolAsyncManager::default().spawn(future)
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  ThreadPoolAsyncManager::default().spawn_with_handle(future)
}

pub fn block_on<F>(f: F) -> <F as Future>::Output
where
  F: Future,
{
  block_on_executor(f)
}
