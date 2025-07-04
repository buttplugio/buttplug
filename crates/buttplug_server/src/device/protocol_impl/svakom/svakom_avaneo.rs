// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};


generic_protocol_setup!(SvakomAvaNeo, "svakom-avaneo");

#[derive(Default)]
pub struct SvakomAvaNeo {}

impl SvakomAvaNeo {
  fn form_hardware_command(
    &self,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [
        0x55,
        0x03,
        0x00,
        0x00,
        if speed == 0 { 0x00 } else { 0x01 },
        speed as u8,
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}

impl ProtocolHandler for SvakomAvaNeo {
  // Note: This protocol used to have a mode byte that was set in cases where multiple commands were
  // sent at the same time. This has been removed in the v10 line, but may cause issues. If we get
  // bug reports on that, we may need to revisit this implementation.

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_id, speed)
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_id, speed)
  }
}
