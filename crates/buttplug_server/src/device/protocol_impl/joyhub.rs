// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use uuid::{Uuid, uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

const JOYHUB_PROTOCOL_UUID: Uuid = uuid!("c0f6785a-0056-4a2a-a2a9-dc7ca4ae2a0d");

generic_protocol_setup!(JoyHub, "joyhub");

#[derive(Default)]
pub struct JoyHub {
  last_cmds: [AtomicU8; 4],
}

impl JoyHub {
  fn form_hardware_command(
    &self,
    index: u32,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_cmds[index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[JOYHUB_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![
          0xa0,
          0x03,
          self.last_cmds[0].load(Ordering::Relaxed),
          self.last_cmds[1].load(Ordering::Relaxed),
          self.last_cmds[2].load(Ordering::Relaxed),
          self.last_cmds[3].load(Ordering::Relaxed),
          0xaa,
        ],
        false,
      )
      .into(),
    ])
  }
}

impl ProtocolHandler for JoyHub {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed.abs() as u32)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index == 4 {
      Ok(vec![
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          vec![
            0xa0,
            0x07,
            if level == 0 { 0x00 } else { 0x01 },
            0x00,
            level as u8,
            0xff,
          ],
          false,
        )
        .into(),
      ])
    } else {
      Ok(vec![
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          vec![0xa0, 0x0d, 0x00, 0x00, level as u8, 0xff],
          false,
        )
        .into(),
      ])
    }
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        if level == 0 {
          vec![0xa0, 0x04, 0x00, 0x00, 0x00, 0x00]
        } else {
          vec![0xa0, 0x04, 0x01, 0x00, 0x01, 0xff]
        },
        false,
      )
          .into(),
    ])
  }

  fn handle_output_led_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        if level == 0 {
          vec![0xa0, 0x14, 0x00, 0x00, 0x00, 0x00]
        } else {
          vec![0xa0, 0x14, 0x01, 0x00, 0x01, 0xff]
        },
        false,
      )
          .into(),
    ])
  }
}
