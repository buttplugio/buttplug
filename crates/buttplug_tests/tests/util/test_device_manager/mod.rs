// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

mod test_device;

mod test_device_comm_manager;

use buttplug_server::device::hardware::HardwareCommand;
use std::time::Duration;
pub use test_device::{TestDevice, TestDeviceChannelHost, TestHardwareEvent};

pub use test_device_comm_manager::{
  //new_bluetoothle_test_device,
  TestDeviceCommunicationManagerBuilder,
  TestDeviceIdentifier,
};

#[allow(dead_code)]
pub async fn check_test_recv_value(
  timeout: &Duration,
  receiver: &mut TestDeviceChannelHost,
  command: HardwareCommand,
) {
  assert_eq!(
    tokio::time::timeout(*timeout, receiver.receiver.recv())
      .await
      .expect("No messages received")
      .expect("Test"),
    command
  );
}
