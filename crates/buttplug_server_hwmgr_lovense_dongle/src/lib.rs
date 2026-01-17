// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[macro_use]
extern crate log;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod lovense_dongle_hardware;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod lovense_dongle_messages;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
mod lovense_dongle_state_machine;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod lovense_hid_dongle_comm_manager;

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use lovense_dongle_hardware::{LovenseDongleHardware, LovenseDongleHardwareConnector};
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use lovense_hid_dongle_comm_manager::{
  LovenseHIDDongleCommunicationManager,
  LovenseHIDDongleCommunicationManagerBuilder,
};
