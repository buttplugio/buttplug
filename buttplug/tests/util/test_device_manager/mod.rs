// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod test_device;
#[cfg(feature = "server")]
mod test_device_comm_manager;

use buttplug::{
  server::device::hardware::HardwareCommand,
  util::stream::{iffy_is_empty_check, recv_now},
};
use std::sync::{Arc, Mutex};
pub use test_device::{
  TestDevice,
  TestDeviceChannelHost,
  TestHardwareConnector,
  TestHardwareEvent,
  TestHardwareNotification,
};
#[cfg(feature = "server")]
pub use test_device_comm_manager::{
  //new_bluetoothle_test_device,
  TestDeviceCommunicationManager,
  TestDeviceCommunicationManagerBuilder,
  TestDeviceIdentifier,
};
use tokio::sync::mpsc::Receiver;

#[allow(dead_code)]
pub fn check_test_recv_value(receiver: &mut TestDeviceChannelHost, command: HardwareCommand) {
  assert_eq!(
    recv_now(&mut receiver.receiver)
      .expect("No messages received")
      .expect("Test"),
    command
  );
}

#[allow(dead_code)]
pub fn check_test_recv_empty(receiver: &Arc<Mutex<Receiver<HardwareCommand>>>) -> bool {
  iffy_is_empty_check(&mut receiver.lock().expect("Test"))
}
