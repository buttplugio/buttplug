// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{
      Endpoint,
    },
  },
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

generic_protocol_setup!(Bananasome, "bananasome");

pub struct Bananasome {
  current_commands: [AtomicU8; 3]
}

impl Default for Bananasome {
  fn default() -> Self {
    Self {
      current_commands: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)]
    }
  }
}

impl Bananasome {
  fn hardware_command(&self) -> Vec<HardwareCommand> {
    vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        self.current_commands[0].load(Ordering::Relaxed) as u8,
        self.current_commands[1].load(Ordering::Relaxed) as u8,
        self.current_commands[2].load(Ordering::Relaxed) as u8,
      ],
      false,
    )
    .into()]
  }
}

impl ProtocolHandler for Bananasome {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn outputs_full_command_set(&self) -> bool {
    true
  }

  fn handle_value_oscillate_cmd(
      &self,
      cmd: &CheckedValueCmdV4,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.current_commands[cmd.feature_index() as usize].store(cmd.value() as u8, Ordering::Relaxed);
    Ok(self.hardware_command())
  }

  fn handle_value_vibrate_cmd(
      &self,
      _cmd: &CheckedValueCmdV4,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(self.hardware_command())
      
  }
}
