// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// WebBluetooth bindings in web-sys are gated behind `web_sys_unstable_apis`,
// which is only enabled for wasm32 targets via .cargo/config.toml rustflags,
// so this crate can only build for wasm. Its wasm-only dependencies are
// likewise scoped to wasm32 in Cargo.toml.
#[cfg(target_arch = "wasm32")]
mod webbluetooth_comm_manager;
#[cfg(target_arch = "wasm32")]
mod webbluetooth_hardware;

#[cfg(target_arch = "wasm32")]
pub use webbluetooth_comm_manager::{
  WebBluetoothCommunicationManager,
  WebBluetoothCommunicationManagerBuilder,
};
#[cfg(target_arch = "wasm32")]
pub use webbluetooth_hardware::WebBluetoothHardwareConnector;
