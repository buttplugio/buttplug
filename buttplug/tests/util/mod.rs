// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod delay_device_communication_manager;
pub use delay_device_communication_manager::DelayDeviceCommunicationManagerBuilder;
mod channel_transport;
pub use channel_transport::*;

#[allow(dead_code)]
pub fn setup_logging() {
  tracing_subscriber::fmt::init();
}
