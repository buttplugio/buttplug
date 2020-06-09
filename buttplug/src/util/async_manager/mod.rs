cfg_if::cfg_if! {
  if #[cfg(feature = "thread_pool_runtime")] {
    mod thread_pool;
    pub use thread_pool::{ThreadPoolAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else if #[cfg(feature = "dummy_runtime")] {
    mod dummy;
    pub use dummy::{DummyAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else if #[cfg(feature = "async_std_runtime")] {
    mod async_std;
    pub use self::async_std::{AsyncStdAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else if #[cfg(feature = "wasm_bindgen_runtime")] {
    mod wasm_bindgen;
    pub use wasm_bindgen::{WasmBindgenAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else {
    std::compile_error!("Please choose a runtime feature: thread_pool_runtime, async_std_runtime, wasm_bindgen_runtime, dummy_runtime");
  }
}