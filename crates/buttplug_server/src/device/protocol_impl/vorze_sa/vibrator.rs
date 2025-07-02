// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{VorzeActions, VorzeDevice};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::ProtocolHandler,
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

pub struct VorzeSAVibrator {
  device_type: VorzeDevice,
}

impl VorzeSAVibrator {
  pub fn new(device_type: VorzeDevice) -> Self {
    Self { device_type }
  }
}

impl ProtocolHandler for VorzeSAVibrator {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![{
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          self.device_type as u8,
          VorzeActions::Vibrate as u8,
          speed as u8,
        ],
        true,
      )
      .into()
    }])
  }
}
