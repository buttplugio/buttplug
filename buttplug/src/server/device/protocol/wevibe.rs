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
    message::{ActuatorType, Endpoint},
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
use std::sync::Arc;

generic_protocol_initializer_setup!(WeVibe, "wevibe");

#[derive(Default)]
pub struct WeVibeInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    debug!("calling WeVibe init");
    hardware
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
        true,
      ))
      .await?;
    hardware
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        true,
      ))
      .await?;
    Ok(Arc::new(WeVibe::default()))
  }
}

#[derive(Default)]
pub struct WeVibe {}

impl ProtocolHandler for WeVibe {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let r_speed_int = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    let r_speed_ext = cmds
      .last()
      .unwrap_or(&None)
      .unwrap_or((ActuatorType::Vibrate, 0u32))
      .1 as u8;
    let data = if r_speed_int == 0 && r_speed_ext == 0 {
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_ext | (r_speed_int << 4),
        0x00,
        0x03,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, data, true).into()])
  }
}
