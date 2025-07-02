use super::VorzeDevice;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::ProtocolHandler,
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

#[derive(Default)]
pub struct VorzeSAPiston {
  previous_position: Arc<AtomicU8>,
}

pub fn get_piston_speed(mut distance: f64, mut duration: f64) -> u8 {
  if distance <= 0f64 {
    return 100;
  }

  if distance > 200f64 {
    distance = 200f64;
  }

  // Convert duration to max length
  duration = 200f64 * duration / distance;

  let mut speed = (duration / 6658f64).powf(-1.21);

  if speed > 100f64 {
    speed = 100f64;
  }

  if speed < 0f64 {
    speed = 0f64;
  }

  speed as u8
}

impl ProtocolHandler for VorzeSAPiston {
  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    feature_id: uuid::Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let previous_position = self.previous_position.load(Ordering::Relaxed);
    let position = position as u8;
    let distance = (previous_position as f64 - position as f64).abs();

    let speed = get_piston_speed(distance, duration as f64);

    self
      .previous_position
      .store(position as u8, Ordering::Relaxed);

    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![VorzeDevice::Piston as u8, position as u8, speed],
      true,
    )
    .into()])
  }
}
