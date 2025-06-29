// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Utility module, for storing types and functions used across other modules in
//! the library.

pub mod async_manager;
pub mod future;
pub mod json;
pub mod stream;

#[cfg(not(feature = "wasm"))]
pub use tokio::time::sleep;
#[cfg(feature = "wasm")]
pub use wasmtimer::tokio::sleep;
