// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};

use async_trait::async_trait;
use uuid::{Uuid, uuid};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use buttplug_core::{errors::ButtplugDeviceError, message::OutputType};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

generic_protocol_initializer_setup!(WeVibeChorus, "wevibe-chorus");

const WEVIBE_CHORUS_PROTOCOL_UUID: Uuid = uuid!("cdeadd1c-b913-4305-a255-bd8834c4e37f");

#[derive(Default)]
pub struct WeVibeChorusInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibeChorusInitializer {
  async fn initialize(
    &mut self,
    _hardware: Arc<Hardware>,
    def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let num_vibrators = def
      .features()
      .values()
      .filter(|x| x.contains_output(OutputType::Vibrate))
      .count() as u8;
    Ok(Arc::new(WeVibeChorus::new(num_vibrators)))
  }
}

pub struct WeVibeChorus {
  num_vibrators: u8,
  speeds: [AtomicU8; 3],
}

impl WeVibeChorus {
  fn new(num_vibrators: u8) -> Self {
    Self {
      num_vibrators,
      speeds: [
        AtomicU8::default(),
        AtomicU8::default(),
        AtomicU8::default(),
      ],
    }
  }

  fn two_motor_packet(&self) -> Vec<u8> {
    let max_vibrators = if self.num_vibrators > 1 { 1 } else { 0 };
    let r_speed_int = self.speeds[0].load(Ordering::Relaxed);
    let r_speed_ext = self.speeds[max_vibrators].load(Ordering::Relaxed);
    if r_speed_int == 0 && r_speed_ext == 0 {
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    } else {
      // Note the motor order is flipped for the Chorus
      let status_byte: u8 =
        (if r_speed_ext == 0 { 0 } else { 2 }) | (if r_speed_int == 0 { 0 } else { 1 });
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_int,
        r_speed_ext,
        status_byte,
        0x00,
        0x00,
      ]
    }
  }

  // The Chorus Pro uses the We-Connect app's 0xe0 command: three speeds (external tip,
  // internal, external base), then one pattern byte per motor (0x03 steady, 0xaa off). Unlike
  // the 0x0f command, this doesn't overwrite the toy's stored stand-alone settings, which 0x0f
  // can leave with motors disabled.
  fn three_motor_packet(&self) -> Vec<u8> {
    // Features keep the two-motor Chorus order (internal, external), with the external base
    // added as feature 2, so feature 0 goes in the middle slot.
    let speeds: Vec<u8> = [1, 0, 2]
      .iter()
      .map(|&i| self.speeds[i].load(Ordering::Relaxed))
      .collect();
    let pattern = |speed: u8| if speed == 0 { 0xaa } else { 0x03 };
    vec![
      0xe0,
      speeds[0],
      speeds[1],
      speeds[2],
      0x00,
      0x00,
      0x00,
      0x00,
      0x00,
      pattern(speeds[0]),
      pattern(speeds[1]),
      pattern(speeds[2]),
      0x00,
      0x00,
      0x00,
      0x00,
    ]
  }
}

impl ProtocolHandler for WeVibeChorus {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let data = if self.num_vibrators > 2 {
      self.three_motor_packet()
    } else {
      self.two_motor_packet()
    };
    Ok(vec![
      HardwareWriteCmd::new(&[WEVIBE_CHORUS_PROTOCOL_UUID], Endpoint::Tx, data, true).into(),
    ])
  }
}
