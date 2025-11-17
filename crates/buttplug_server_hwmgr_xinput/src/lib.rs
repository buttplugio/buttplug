// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(target_os = "windows")]
#[macro_use]
extern crate log;

#[cfg(target_os = "windows")]
#[macro_use]
extern crate strum_macros;

#[cfg(target_os = "windows")]
mod xinput_device_comm_manager;
#[cfg(target_os = "windows")]
mod xinput_hardware;

#[cfg(target_os = "windows")]
pub use xinput_device_comm_manager::{
  XInputDeviceCommunicationManager,
  XInputDeviceCommunicationManagerBuilder,
};
