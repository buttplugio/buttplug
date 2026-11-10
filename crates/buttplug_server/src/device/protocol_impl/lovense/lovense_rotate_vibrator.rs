// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{form_rotate_with_direction_command, form_vibrate_command};

use crate::device::{
  hardware::{Hardware, HardwareCommand},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::{errors::ButtplugDeviceError, message::InputReadingV4};
use futures::future::BoxFuture;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

pub struct LovenseRotateVibrator {
  clockwise: AtomicBool,
}

impl Default for LovenseRotateVibrator {
  fn default() -> Self {
    Self {
      // Convention: positive speed is the power-on direction, since Lovense can't report it.
      clockwise: AtomicBool::new(true),
    }
  }
}

impl ProtocolHandler for LovenseRotateVibrator {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    super::keepalive_strategy()
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    form_vibrate_command(feature_id, speed)
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let change_direction = if speed == 0 {
      false
    } else {
      let clockwise = speed > 0;
      self.clockwise.swap(clockwise, Ordering::Relaxed) != clockwise
    };
    form_rotate_with_direction_command(speed.unsigned_abs(), change_direction)
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

#[cfg(test)]
mod tests {
  use super::LovenseRotateVibrator;
  use crate::device::{hardware::HardwareCommand, protocol::ProtocolHandler};
  use uuid::Uuid;

  fn rotate_commands(protocol: &LovenseRotateVibrator, speed: i32) -> Vec<Vec<u8>> {
    protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), speed)
      .expect("command should be valid")
      .into_iter()
      .map(|cmd| match cmd {
        HardwareCommand::Write(write_cmd) => write_cmd.data().clone(),
        _ => panic!("Expected a write command"),
      })
      .collect()
  }

  fn contains_rotate_change(commands: &[Vec<u8>]) -> bool {
    commands.iter().any(|data| data == b"RotateChange;")
  }

  #[test]
  fn initial_positive_speed_does_not_toggle() {
    let protocol = LovenseRotateVibrator::default();
    let commands = rotate_commands(&protocol, 20);
    assert!(!contains_rotate_change(&commands));
  }

  #[test]
  fn repeated_negative_speeds_toggle_once() {
    let protocol = LovenseRotateVibrator::default();
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -10)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -5)));
  }

  #[test]
  fn negative_then_positive_toggles_back() {
    let protocol = LovenseRotateVibrator::default();
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(contains_rotate_change(&rotate_commands(&protocol, 20)));
  }

  #[test]
  fn zero_speed_does_not_toggle() {
    let protocol = LovenseRotateVibrator::default();
    assert!(!contains_rotate_change(&rotate_commands(&protocol, 0)));
    assert!(contains_rotate_change(&rotate_commands(&protocol, -20)));
    assert!(!contains_rotate_change(&rotate_commands(&protocol, -10)));
  }

  #[test]
  fn toggle_writes_do_not_share_command_ids() {
    let protocol = LovenseRotateVibrator::default();
    let first_call = protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), -10)
      .expect("command should be valid");
    let second_call = protocol
      .handle_output_rotate_cmd(0, Uuid::nil(), 10)
      .expect("command should be valid");

    let is_rotate_change = |cmd: &&HardwareCommand| matches!(cmd, HardwareCommand::Write(w) if w.data() == b"RotateChange;");
    let is_rotate_speed = |cmd: &&HardwareCommand| matches!(cmd, HardwareCommand::Write(w) if w.data().starts_with(b"Rotate:"));

    let first_toggle = first_call
      .iter()
      .find(is_rotate_change)
      .expect("Expected a RotateChange command in the first call");
    let second_toggle = second_call
      .iter()
      .find(is_rotate_change)
      .expect("Expected a RotateChange command in the second call");
    let speed_write = second_call
      .iter()
      .find(is_rotate_speed)
      .expect("Expected a Rotate speed command");

    assert!(!first_toggle.overlaps(second_toggle));
    assert!(!first_toggle.overlaps(speed_write));
    assert!(!second_toggle.overlaps(speed_write));
  }
}
