// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::form_vibrate_command;
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::InputReadingV4,
};
use buttplug_server_device_config::Endpoint;
use futures::future::BoxFuture;
use std::sync::{atomic::AtomicU32, Arc};
use uuid::Uuid;

#[derive(Default)]
pub struct LovenseMultiActuator {
  _vibrator_values: Vec<AtomicU32>,
}

impl LovenseMultiActuator {
  pub fn new(num_vibrators: u32) -> Self {
    Self {
      _vibrator_values: std::iter::repeat_with(|| AtomicU32::new(0))
        .take(num_vibrators as usize)
        .collect(),
    }
  }
}

impl ProtocolHandler for LovenseMultiActuator {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    super::keepalive_strategy()
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let lovense_cmd = format!("Vibrate{}:{};", feature_index + 1, speed)
      .as_bytes()
      .to_vec();
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      lovense_cmd,
      false,
    )
    .into()])
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    form_vibrate_command(feature_id, speed)
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'static, Result<InputReadingV4, ButtplugDeviceError>> {
    super::handle_battery_level_cmd(device_index, device, feature_index, feature_id)
  }
}
