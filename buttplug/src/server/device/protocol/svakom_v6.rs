// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::Endpoint,
    message::{ActuatorType, ActuatorType::Vibrate},
  },
  generic_protocol_initializer_setup,
  server::device::{
    configuration::UserDeviceDefinition,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      ProtocolCommunicationSpecifier,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
      UserDeviceIdentifier,
    },
  },
};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

generic_protocol_initializer_setup!(SvakomV6, "svakom-v6");

#[derive(Default)]
pub struct SvakomV6Initializer {}

#[async_trait]
impl ProtocolInitializer for SvakomV6Initializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomV6::new()))
  }
}

pub struct SvakomV6 {
  last_cmds: RwLock<Vec<(ActuatorType, u32)>>,
}

impl SvakomV6 {
  fn new() -> Self {
    let last_cmds = RwLock::new(vec![]);
    Self { last_cmds }
  }
}

impl ProtocolHandler for SvakomV6 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let last_commands = self.last_cmds.read().expect("Locks should work").clone();
    let mut hcmds = Vec::new();

    let vibes = commands
      .iter()
      .filter(|c| c.is_some_and(|c| c.0 == Vibrate))
      .map(|c| c.unwrap_or((Vibrate, 0)))
      .collect::<Vec<(ActuatorType, u32)>>();
    let last_vibes = last_commands
      .iter()
      .filter(|c| c.0 == Vibrate)
      .map(|c| (c.0, c.1))
      .collect::<Vec<(ActuatorType, u32)>>();

    if vibes.len() > 0 {
      let mut changed = last_vibes.len() != vibes.len();
      let vibe1 = vibes[0].1;
      if !changed && vibes[0].1 != last_vibes[0].1 {
        changed = true;
      }
      let mut vibe2 = vibes[0].1;
      if vibes.len() > 1 {
        vibe2 = vibes[1].1;
        if !changed && vibes[1].1 != last_vibes[1].1 {
          changed = true;
        }
      }
      if changed {
        hcmds.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [
              0x55,
              0x03,
              if (vibe1 > 0 && vibe2 > 0) || vibe1 == vibe2 {
                0x00
              } else if vibe1 > 0 {
                0x01
              } else {
                0x02
              },
              0x00,
              if vibe1 == vibe2 && vibe1 == 0 {
                0x00
              } else {
                0x01
              },
              vibe1.max(vibe2) as u8,
              0x00,
            ]
            .to_vec(),
            false,
          )
          .into(),
        );
      }
    }

    if vibes.len() > 2 {
      let mut changed = last_vibes.len() != vibes.len();
      let vibe3 = vibes[2].1;
      if !changed && vibes[2].1 != last_vibes[2].1 {
        changed = true;
      }
      if changed {
        hcmds.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [
              0x55,
              0x07,
              0x00,
              0x00,
              if vibe3 == 0 { 0x00 } else { 0x01 },
              vibe3 as u8,
              0x00,
            ]
            .to_vec(),
            false,
          )
          .into(),
        );
      }
    }

    let mut command_writer = self.last_cmds.write().expect("Locks should work");
    *command_writer = commands
      .iter()
      .filter(|c| c.is_some())
      .map(|c| c.unwrap_or((Vibrate, 0)))
      .collect::<Vec<(ActuatorType, u32)>>();
    Ok(hcmds)
  }
}
