// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub mod lovense_dongle_hardware;
mod lovense_dongle_messages;
mod lovense_dongle_state_machine;
pub mod lovense_hid_dongle_comm_manager;
pub mod lovense_serial_dongle_comm_manager;

pub use lovense_dongle_hardware::{LovenseDongleHardware, LovenseDongleHardwareConnector};
pub use lovense_hid_dongle_comm_manager::{
  LovenseHIDDongleCommunicationManager,
  LovenseHIDDongleCommunicationManagerBuilder,
};
pub use lovense_serial_dongle_comm_manager::{
  LovenseSerialDongleCommunicationManager,
  LovenseSerialDongleCommunicationManagerBuilder,
};
