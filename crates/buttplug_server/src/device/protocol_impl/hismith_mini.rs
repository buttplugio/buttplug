// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, message::OutputType};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};
use uuid::{Uuid, uuid};

const HISMITH_MINI_PROTOCOL_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");
const HISMITH_MINI_ROTATE_DIRECTIOM_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");

// Servo machines (HISMITH S2 / Servo) expose these Position features, by feature index. The
// opcodes come from the official app's CC command set.
const POSITION_FEATURE_STROKE_START: u32 = 1; // opcode 0x07
const POSITION_FEATURE_STROKE_END: u32 = 2; // opcode 0x08
const POSITION_FEATURE_DIRECT: u32 = 3; // opcode 0x0A, move to this depth now
const POSITION_FEATURE_SMOOTHNESS: u32 = 4; // opcode 0x09, 0-10

// The official app never lets the stroke end get closer than this to the stroke start.
const MIN_STROKE_GAP: u8 = 10;

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct HismithMiniIdentifierFactory {}

  impl ProtocolIdentifierFactory for HismithMiniIdentifierFactory {
    fn identifier(&self) -> &str {
      "hismith-mini"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::HismithMiniIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct HismithMiniIdentifier {}

#[async_trait]
impl ProtocolIdentifier for HismithMiniIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(
        HISMITH_MINI_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        128,
        500,
      ))
      .await?;

    let identifier = result
      .data()
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<String>();
    info!("Hismith Device Identifier: {}", identifier);

    Ok((
      UserDeviceIdentifier::new(hardware.address(), "hismith-mini", &Some(identifier)),
      Box::new(HismithMiniInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct HismithMiniInitializer {}

#[async_trait]
impl ProtocolInitializer for HismithMiniInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(HismithMini {
      stroke_start: AtomicU8::new(0),
      stroke_end: AtomicU8::new(100),
      dual_vibe: device_definition
        .features()
        .values()
        .filter(|x| x.contains_output(OutputType::Vibrate))
        .count()
        >= 2,
      second_constrict: device_definition
        .features()
        .values()
        .position(|x| x.contains_output(OutputType::Constrict))
        .unwrap_or(0)
        == 1,
    }))
  }
}

pub struct HismithMini {
  dual_vibe: bool,
  second_constrict: bool,
  // Last stroke window we sent (the machine boots with 0-100 unless the app changed it). Used to
  // keep end >= start + MIN_STROKE_GAP like the official app does.
  stroke_start: AtomicU8,
  stroke_end: AtomicU8,
}

impl Default for HismithMini {
  fn default() -> Self {
    Self {
      dual_vibe: false,
      second_constrict: false,
      stroke_start: AtomicU8::new(0),
      stroke_end: AtomicU8::new(100),
    }
  }
}

impl ProtocolHandler for HismithMini {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x03;
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if !self.dual_vibe || feature_index == 1 {
      0x05
    } else {
      0x03
    };
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if self.second_constrict { 0x05 } else { 0x03 };
    let speed: u8 = level as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0xCC,
          0x03,
          speed.unsigned_abs() as u8,
          speed.unsigned_abs() as u8 + 3,
        ],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &[HISMITH_MINI_ROTATE_DIRECTIOM_UUID],
        Endpoint::Tx,
        vec![
          0xCC,
          0x01,
          if speed >= 0 { 0xc0 } else { 0xc1 },
          if speed >= 0 { 0xc1 } else { 0xc2 },
        ],
        false,
      )
      .into(),
    ])
  }

  // Depth/stroke control of the servo machines (HISMITH S2 / Servo), values are 0-100 percent of
  // the machine's travel:
  //   index 1 -> stroke start (inner limit), 0x07
  //   index 2 -> stroke end (outer limit), 0x08
  //   index 3 -> direct position, move to this depth now, 0x0A
  //   index 4 -> smoothness, 0x09 (0-10)
  // The stroke window is kept at least MIN_STROKE_GAP wide, as the official app does.
  fn handle_output_position_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let (idx, value): (u8, u8) = match feature_index {
      POSITION_FEATURE_STROKE_START => {
        let end = self.stroke_end.load(Ordering::Relaxed);
        let start = (position.min(100) as u8).min(end.saturating_sub(MIN_STROKE_GAP));
        self.stroke_start.store(start, Ordering::Relaxed);
        (0x07, start)
      }
      POSITION_FEATURE_DIRECT => (0x0A, position.min(100) as u8),
      POSITION_FEATURE_SMOOTHNESS => (0x09, position.min(10) as u8),
      // POSITION_FEATURE_STROKE_END and anything unexpected
      _ => {
        let start = self.stroke_start.load(Ordering::Relaxed);
        let end = (position.min(100) as u8).max((start + MIN_STROKE_GAP).min(100));
        self.stroke_end.store(end, Ordering::Relaxed);
        (0x08, end)
      }
    };

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, value, value + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_spray_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xcc, 0x0b, 0x01, 0x0c],
        false,
      )
      .into(),
    ])
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn written(cmds: Vec<HardwareCommand>) -> Vec<u8> {
    match cmds.into_iter().next().unwrap() {
      HardwareCommand::Write(w) => w.data().clone(),
      _ => panic!("expected a write command"),
    }
  }

  #[test]
  fn stroke_start_uses_opcode_07() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(1, Uuid::nil(), 20)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x07, 0x14, 0x1B]);
  }

  #[test]
  fn stroke_end_uses_opcode_08() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 80)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x50, 0x58]);
  }

  #[test]
  fn stroke_position_is_clamped_to_100() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 250)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x64, 0x6C]);
  }

  #[test]
  fn speed_command_is_unchanged() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_oscillate_cmd(0, Uuid::nil(), 50)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x03, 0x32, 0x35]);
  }

  #[test]
  fn direct_position_uses_opcode_0a() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(3, Uuid::nil(), 50)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x0A, 0x32, 0x3C]);
  }

  #[test]
  fn smoothness_uses_opcode_09_and_clamps_to_10() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(4, Uuid::nil(), 7)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x09, 0x07, 0x10]);
    let cmds = handler
      .handle_output_position_cmd(4, Uuid::nil(), 99)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x09, 0x0A, 0x13]);
  }

  #[test]
  fn stroke_start_cannot_pass_end_minus_gap() {
    let handler = HismithMini::default();
    // end defaults to 100, so start is limited to 90
    let cmds = handler
      .handle_output_position_cmd(1, Uuid::nil(), 100)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x07, 0x5A, 0x61]);
  }

  #[test]
  fn stroke_end_cannot_pass_start_plus_gap() {
    let handler = HismithMini::default();
    handler
      .handle_output_position_cmd(1, Uuid::nil(), 40)
      .unwrap();
    // start is 40, so end is limited to 50
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 0)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x32, 0x3A]);
  }
}
