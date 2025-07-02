// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

generic_protocol_setup!(ServeU, "serveu");

#[derive(Default)]
pub struct ServeU {
  last_position: Arc<AtomicU8>,
}

impl ProtocolHandler for ServeU {
  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let last_pos = self.last_position.load(Ordering::Relaxed);
    // Need to get "units" (abstracted steps 0-100) per second, so calculate how far we need to move over our goal duration.
    let goal_pos = position as u8;
    self.last_position.store(goal_pos, Ordering::Relaxed);
    let speed_threshold =
      ((((goal_pos as i8) - last_pos as i8).abs()) as f64 / ((duration as f64) / 1000f64)).ceil();

    let speed = if speed_threshold <= 0.00001 {
      // Stop device
      0
    } else if speed_threshold <= 50.0 {
      (speed_threshold / 2.0).ceil() as u8
    } else if speed_threshold <= 750.0 {
      ((speed_threshold - 50.0) / 4.0).ceil() as u8 + 25u8
    } else if speed_threshold <= 2000.0 {
      ((speed_threshold - 750.0) / 25.0).ceil() as u8 + 200u8
    } else {
      // If we're going faster than 2000u/s, just return max value (0xFA)
      0xFA
    };

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0x01, goal_pos, speed],
      false,
    )
    .into()])
  }
}
