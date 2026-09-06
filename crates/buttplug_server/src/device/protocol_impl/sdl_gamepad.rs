// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::Endpoint;
use byteorder::LittleEndian;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use byteorder::WriteBytesExt;
use std::sync::atomic::{AtomicU16, Ordering};

generic_protocol_setup!(SdlGamepad, "sdl-gamepad");

/// SDL3 gamepad rumble protocol.
///
/// Like XInput, every vibrate command carries the *complete* motor state: the
/// handler keeps the last-set speed of both motors and packs both u16 values
/// (little-endian) into every write packet, so every `write_value` is a full
/// command and no batching/drain step is needed on the hardware side.
///
/// Packet layout (4 bytes, little-endian):
///   bytes 0-1: low-frequency motor speed (feature 0), 0-65535
///   bytes 2-3: high-frequency motor speed (feature 1), 0-65535
#[derive(Default)]
pub struct SdlGamepad {
  speeds: [AtomicU16; 2],
}

impl ProtocolHandler for SdlGamepad {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index > 1 {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "SdlGamepad".to_owned(),
        format!("SDL gamepad only has 2 vibrate features, got index {feature_index}"),
      ));
    }
    self.speeds[feature_index as usize].store(speed as u16, Ordering::Relaxed);
    let mut cmd = vec![];
    if cmd
      .write_u16::<LittleEndian>(self.speeds[0].load(Ordering::Relaxed))
      .is_err()
      || cmd
        .write_u16::<LittleEndian>(self.speeds[1].load(Ordering::Relaxed))
        .is_err()
    {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "SdlGamepad".to_owned(),
        "Cannot convert SDL gamepad value for processing".to_owned(),
      ));
    }
    Ok(vec![
      HardwareWriteCmd::new(&[_feature_id], Endpoint::Tx, cmd, false).into(),
    ])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn vibrate(handler: &SdlGamepad, feature_index: u32, speed: u32) -> Vec<u8> {
    let cmds = handler
      .handle_output_vibrate_cmd(feature_index, uuid::Uuid::new_v4(), speed)
      .expect("vibrate command should build");
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
      HardwareCommand::Write(write_cmd) => {
        assert_eq!(write_cmd.endpoint(), Endpoint::Tx);
        write_cmd.data().clone()
      }
      _ => panic!("expected a write command"),
    }
  }

  #[test]
  fn sdl_gamepad_packs_both_motor_states() {
    let handler = SdlGamepad::default();

    // Feature 0 (low motor) only: high motor stays 0.
    let packet = vibrate(&handler, 0, 0x8000);
    assert_eq!(packet, vec![0x00, 0x80, 0x00, 0x00]);

    // Feature 1 (high motor) now set: packet must carry BOTH stored speeds,
    // proving the handler is stateful across commands.
    let packet = vibrate(&handler, 1, 0x7fff);
    assert_eq!(packet, vec![0x00, 0x80, 0xff, 0x7f]);

    // Updating feature 0 again keeps feature 1's stored speed.
    let packet = vibrate(&handler, 0, 0x1234);
    assert_eq!(packet, vec![0x34, 0x12, 0xff, 0x7f]);

    // Speeds clamp to u16 in the same way as XInput (store as u16).
    let packet = vibrate(&handler, 0, 0xffff);
    assert_eq!(packet, vec![0xff, 0xff, 0xff, 0x7f]);
  }

  #[test]
  fn sdl_gamepad_rejects_out_of_range_feature() {
    let handler = SdlGamepad::default();
    assert!(
      handler
        .handle_output_vibrate_cmd(2, uuid::Uuid::new_v4(), 100)
        .is_err()
    );
  }
}
