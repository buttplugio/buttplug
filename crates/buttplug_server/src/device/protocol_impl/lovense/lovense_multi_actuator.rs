// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::{errors::ButtplugDeviceError, message::InputReadingV4};
use buttplug_server_device_config::Endpoint;
use futures::future::BoxFuture;
use std::sync::atomic::Ordering;
use std::sync::{Arc, atomic::AtomicU32};
use uuid::{Uuid, uuid};

const LOVENSE_MPLY_PROTOCOL_UUID: Uuid = uuid!("7925d93b-15d0-4c59-bb5b-9779ec6c04e4");
#[derive(Default)]
pub struct LovenseMultiActuator {
  vibrator_values: Vec<AtomicU32>,
}

impl LovenseMultiActuator {
  pub fn new(num_vibrators: u32) -> Self {
    Self {
      vibrator_values: std::iter::repeat_with(|| AtomicU32::new(0))
        .take(num_vibrators as usize)
        .collect(),
    }
  }

  fn form_packet(&self) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[LOVENSE_MPLY_PROTOCOL_UUID],
        Endpoint::Tx,
        format!(
          "Mply:{};",
          self
            .vibrator_values
            .iter()
            .map(|v| v.load(Ordering::Relaxed).to_string())
            .collect::<Vec<String>>()
            .join(":")
        )
        .as_bytes()
        .to_vec(),
        false,
      )
      .into(),
    ])
  }
}

impl ProtocolHandler for LovenseMultiActuator {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    super::keepalive_strategy()
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.vibrator_values[feature_index as usize].store(speed, Ordering::Relaxed);
    self.form_packet()
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.vibrator_values[feature_index as usize].store(speed.abs() as u32, Ordering::Relaxed);
    self.form_packet()
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
