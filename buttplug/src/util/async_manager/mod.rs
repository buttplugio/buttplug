// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

cfg_if::cfg_if! {
  if #[cfg(feature = "dummy-runtime")] {
    mod dummy;
    pub use dummy::{DummyAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else if #[cfg(feature = "wasm-bindgen-runtime")] {
    mod wasm_bindgen;
    pub use self::wasm_bindgen::{WasmBindgenAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  } else if #[cfg(feature = "tokio-runtime")] {
    mod tokio;
    pub use self::tokio::{TokioAsyncManager as AsyncManager, spawn, spawn_with_handle, block_on};
  }
  else {
    std::compile_error!("Please choose a runtime feature: tokio-runtime, wasm-bindgen-runtime, dummy-runtime");
  }
}
