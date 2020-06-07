#[cfg(feature = "thread_pool_runtime")]
mod thread_pool;
#[cfg(feature="dummy_runtime")]
mod dummy;
#[cfg(feature="async_std_runtime")]
mod async_std;

#[cfg(feature = "thread_pool_runtime")]
pub use thread_pool::{ThreadPoolAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
#[cfg(feature="dummy_runtime")]
pub use dummy::{DummyAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
#[cfg(feature="async_std_runtime")]
pub use self::async_std::{AsyncStdAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};

/*
use futures::{
  future::{Future, RemoteHandle},
  task::{FutureObj, Spawn, SpawnError, SpawnExt},
};

#[cfg(feature = "thread_pool_runtime")]
//pub use thread_pool::{ThreadPoolAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
#[cfg(feature="dummy_runtime")]
pub use dummy::{DummyAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
#[cfg(feature="async_std_runtime")]
pub use async_executors::AsyncStd as AsyncManager;

pub fn spawn<Fut>(future: Fut) -> Result<(), SpawnError>
where
  Fut: Future<Output = ()> + Send + 'static,
{
  AsyncManager::default().spawn(future)
}

pub fn spawn_with_handle<Fut>(future: Fut) -> Result<RemoteHandle<Fut::Output>, SpawnError>
where
  Fut: Future + Send + 'static,
  Fut::Output: Send,
{
  AsyncManager::default().spawn_with_handle(future)
}

pub fn block_on<F>(f: F) -> <F as Future>::Output
where
  F: Future,
{
  block_on_executor(f)
}
*/
