// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::core::message::ActuatorType::{Oscillate, Vibrate};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  },
};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};
generic_protocol_initializer_setup!(SvakomV5, "svakom-v5");

#[derive(Default)]
pub struct SvakomV5Initializer {}

#[async_trait]
impl ProtocolInitializer for SvakomV5Initializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(SvakomV5::new()))
  }
}

pub struct SvakomV5 {
  last_cmds: RwLock<Vec<(ActuatorType, u32)>>,
}

impl SvakomV5 {
  fn new() -> Self {
    let last_cmds = RwLock::new(vec![]);
    Self { last_cmds }
  }
}

impl ProtocolHandler for SvakomV5 {
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
            ]
            .to_vec(),
            false,
          )
          .into(),
        );
      }
    }

    let oscs = commands
      .iter()
      .filter(|c| c.is_some_and(|c| c.0 == Oscillate))
      .map(|c| c.unwrap_or((Oscillate, 0)))
      .collect::<Vec<(ActuatorType, u32)>>();
    let last_oscs = last_commands
      .iter()
      .filter(|c| c.0 == Oscillate)
      .map(|c| (c.0, c.1))
      .collect::<Vec<(ActuatorType, u32)>>();
    if oscs.len() > 0 {
      let mut changed = oscs.len() != last_oscs.len();
      if !changed && oscs[0].1 != last_oscs[0].1 {
        changed = true;
      }

      if changed {
        hcmds.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            [0x55, 0x09, 0x00, 0x00, oscs[0].1 as u8, 0x00].to_vec(),
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
