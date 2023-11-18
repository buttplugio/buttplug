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
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct PatooIdentifierFactory {}

  impl ProtocolIdentifierFactory for PatooIdentifierFactory {
    fn identifier(&self) -> &str {
      "patoo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::PatooIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct PatooIdentifier {}

#[async_trait]
impl ProtocolIdentifier for PatooIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    // Patoo Love devices have wildcarded names of ([A-Z]+)\d*
    // Force the identifier lookup to the non-numeric portion
    let c: Vec<char> = hardware.name().chars().collect();
    let mut i = 0;
    while i < c.len() && !c[i].is_ascii_digit() {
      i += 1;
    }
    let name: String = c[0..i].iter().collect();
    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "Patoo",
        &ProtocolAttributesType::Identifier(name),
      ),
      Box::new(PatooInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct PatooInitializer {}

#[async_trait]
impl ProtocolInitializer for PatooInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Patoo::default()))
  }
}

#[derive(Default)]
pub struct Patoo {}

impl ProtocolHandler for Patoo {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    // Default to vibes
    let mut mode: u8 = 4u8;

    // Use vibe 1 as speed
    let mut speed = cmds[0].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
    if speed == 0 {
      mode = 0;

      // If we have a second vibe and it's not also 0, use that
      if cmds.len() > 1 {
        speed = cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8;
        if speed != 0 {
          mode |= 0x80;
        }
      }
    } else if cmds.len() > 1 && cmds[1].unwrap_or((ActuatorType::Vibrate, 0)).1 as u8 != 0 {
      // Enable second vibe if it's not at 0
      mode |= 0x80;
    }

    msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, vec![speed], true).into());
    msg_vec.push(HardwareWriteCmd::new(Endpoint::TxMode, vec![mode], true).into());

    Ok(msg_vec)
  }
}
