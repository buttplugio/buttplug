// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{VorzeActions, VorzeDevice};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::ProtocolHandler
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

pub struct VorzeSASingleRotator {
  device_type: VorzeDevice,
}

impl VorzeSASingleRotator {
  pub fn new(device_type: VorzeDevice) -> Self {
    Self { device_type }
  }
}

impl ProtocolHandler for VorzeSASingleRotator {
  fn handle_rotation_with_direction_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
    clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let clockwise = if clockwise { 1u8 } else { 0 };
    let data: u8 = (clockwise) << 7 | (speed as u8);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![self.device_type as u8, VorzeActions::Rotate as u8, data],
      true,
    )
    .into()])
  }
}
