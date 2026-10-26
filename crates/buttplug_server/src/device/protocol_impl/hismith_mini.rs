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
use std::sync::Arc;
use uuid::{Uuid, uuid};

const HISMITH_MINI_PROTOCOL_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");
const HISMITH_MINI_ROTATE_DIRECTIOM_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");

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

#[derive(Default)]
pub struct HismithMini {
  dual_vibe: bool,
  second_constrict: bool,
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

  // Stroke window of the servo machines (HISMITH S2 / Servo). Feature index 1 sets the start of
  // the stroke (opcode 0x07), index 2 sets the end of the stroke (opcode 0x08). Both are
  // percentages of the machine's travel (0-100). The official app keeps end - start >= 10.
  fn handle_output_position_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if feature_index == 1 { 0x07 } else { 0x08 };
    let position: u8 = position.min(100) as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, position, position + idx],
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
}
