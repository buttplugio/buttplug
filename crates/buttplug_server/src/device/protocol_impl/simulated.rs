// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::protocol::{ProtocolHandler, generic_protocol_setup};
use buttplug_core::errors::ButtplugDeviceError;
use crate::device::hardware::HardwareCommand;
use uuid::Uuid;

generic_protocol_setup!(SimulatedProtocol, "simulated");

#[derive(Default)]
pub struct SimulatedProtocol {}

impl ProtocolHandler for SimulatedProtocol {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _value: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_spray_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_output_led_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _color: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _value: u32,
    _duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![])
  }
}
