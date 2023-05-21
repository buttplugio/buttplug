// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

generic_protocol_initializer_setup!(VorzeSA, "vorze-sa");

#[derive(Default)]
pub struct VorzeSAInitializer {}

#[async_trait]
impl ProtocolInitializer for VorzeSAInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let hwname = hardware.name().to_ascii_lowercase();
    let device_type = if hwname.contains("cycsa") {
      VorzeDevice::Cyclone
    } else if hwname.contains("ufo-tw") {
      VorzeDevice::UfoTw
    } else if hwname.contains("ufo") {
      VorzeDevice::Ufo
    } else if hwname.contains("bach") {
      VorzeDevice::Bach
    } else if hwname.contains("rocket") {
      VorzeDevice::Rocket
    } else if hwname.contains("piston") {
      VorzeDevice::Piston
    } else {
      return Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
        "No protocol implementation for Vorze Device {}",
        hardware.name()
      )));
    };
    Ok(Arc::new(VorzeSA::new(device_type)))
  }
}

pub struct VorzeSA {
  previous_position: Arc<AtomicU8>,
  device_type: VorzeDevice,
}

impl VorzeSA {
  pub fn new(device_type: VorzeDevice) -> Self {
    Self {
      previous_position: Arc::new(AtomicU8::new(0)),
      device_type,
    }
  }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum VorzeDevice {
  Bach = 6,
  Piston = 3,
  Cyclone = 1,
  Rocket = 7,
  Ufo = 2,
  UfoTw = 5,
}

#[repr(u8)]
enum VorzeActions {
  Rotate = 1,
  Vibrate = 3,
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

impl ProtocolHandler for VorzeSA {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![{
      HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          self.device_type as u8,
          VorzeActions::Vibrate as u8,
          scalar as u8,
        ],
        true,
      )
      .into()
    }])
  }

  fn handle_rotate_cmd(
    &self,
    cmds: &[Option<(u32, bool)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if cmds.len() == 1 {
      if let Some((speed, clockwise)) = cmds[0] {
        let data: u8 = (clockwise as u8) << 7 | (speed as u8);
        Ok(vec![HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![self.device_type as u8, VorzeActions::Rotate as u8, data],
          true,
        )
        .into()])
      } else {
        Ok(vec![])
      }
    } else {
      let mut data_left = 0u8;
      let mut data_right = 0u8;
      let mut changed = false;
      if let Some((speed, clockwise)) = cmds[0] {
        data_left = (clockwise as u8) << 7 | (speed as u8);
        changed = true;
      }
      if let Some((speed, clockwise)) = cmds[1] {
        data_right = (clockwise as u8) << 7 | (speed as u8);
        changed = true;
      }
      if changed {
        Ok(vec![HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![self.device_type as u8, data_left, data_right],
          true,
        )
        .into()])
      } else {
        Ok(vec![])
      }
    }
  }

  fn handle_linear_cmd(
    &self,
    msg: message::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let v = msg.vectors()[0].clone();

    let previous_position = self.previous_position.load(Ordering::SeqCst);
    let position = v.position() * 200f64;
    let distance = (previous_position as f64 - position).abs();

    let speed = get_piston_speed(distance, v.duration() as f64);

    self
      .previous_position
      .store(position as u8, Ordering::SeqCst);

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![self.device_type as u8, position as u8, speed as u8],
      true,
    )
    .into()])
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    msg: message::VorzeA10CycloneCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_rotate_cmd(&[Some((msg.speed(), msg.clockwise()))])
  }
}
