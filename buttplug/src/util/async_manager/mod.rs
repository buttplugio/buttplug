#[cfg(all(not(feature="async_std_runtime"), not(feature="tokio_runtime"), not(feature="dummy_runtime"), feature="thread_pool_runtime"))]
mod thread_pool;
#[cfg(all(not(feature="async_std_runtime"), not(feature="tokio_runtime"), not(feature="thread_pool_runtime"), feature="dummy_runtime"))]
mod dummy;

#[cfg(all(not(feature="async_std_runtime"), not(feature="tokio_runtime"), not(feature="dummy_runtime"), feature="thread_pool_runtime"))]
pub use thread_pool::{ThreadPoolAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
#[cfg(all(not(feature="async_std_runtime"), not(feature="tokio_runtime"), not(feature="thread_pool_runtime"), feature="dummy_runtime"))]
pub use dummy::{DummyAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
