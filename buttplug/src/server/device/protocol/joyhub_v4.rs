// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  generic_protocol_initializer_setup,
  server::device::{
    configuration::UserDeviceIdentifier,
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use std::time::Duration;

generic_protocol_initializer_setup!(JoyHubV4, "joyhub-v4");

async fn delayed_constrict_handler(device: Arc<Hardware>, scalar: u8) {
  sleep(Duration::from_millis(25)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xa0,
        0x07,
        if scalar == 0 { 0x00 } else { 0x01 },
        0x00,
        scalar,
        0xff,
      ],
      false,
    ))
    .await;
  if res.is_err() {
    error!("Delayed JoyHub Constrict command error: {:?}", res.err());
  }
}
fn vibes_changed(
  old_commands_lock: &RwLock<Vec<Option<(ActuatorType, u32)>>>,
  new_commands: &[Option<(ActuatorType, u32)>],
  exclude: Vec<usize>,
) -> bool {
  let old_commands = old_commands_lock.read().expect("locks should work");
  if old_commands.len() != new_commands.len() {
    return true;
  }

  for i in 0..old_commands.len() {
    if exclude.contains(&i) {
      continue;
    }
    if let Some(ocmd) = old_commands[i] {
      if let Some(ncmd) = new_commands[i] {
        if ocmd.1 != ncmd.1 {
          return true;
        }
      }
    }
  }
  false
}

#[derive(Default)]
pub struct JoyHubV4Initializer {}

#[async_trait]
impl ProtocolInitializer for JoyHubV4Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(JoyHubV4::new(hardware)))
  }
}

pub struct JoyHubV4 {
  device: Arc<Hardware>,
  last_cmds: RwLock<Vec<Option<(ActuatorType, u32)>>>,
}

impl JoyHubV4 {
  fn new(device: Arc<Hardware>) -> Self {
    let last_cmds = RwLock::new(vec![]);
    Self { device, last_cmds }
  }
}

impl ProtocolHandler for JoyHubV4 {
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
    let cmd1 = commands[0];
    let cmd2 = if commands.len() > 1 {
      commands[1]
    } else {
      None
    };
    let cmd3 = if commands.len() > 2 {
      commands[2]
    } else {
      None
    };

    if let Some(cmd) = cmd3 {
      if cmd.0 == ActuatorType::Constrict {
        if vibes_changed(&self.last_cmds, commands, vec![2usize]) {
          let dev = self.device.clone();
          async_manager::spawn(async move { delayed_constrict_handler(dev, cmd.1 as u8).await });
        } else {
          let mut command_writer = self.last_cmds.write().expect("Locks should work");
          *command_writer = commands.to_vec();

          return Ok(vec![HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![
              0xa0,
              0x07,
              if cmd.1 == 0 { 0x00 } else { 0x01 },
              0x00,
              cmd.1 as u8,
              0xff,
            ],
            false,
          )
          .into()]);
        }
      }
    }

    let mut command_writer = self.last_cmds.write().expect("Locks should work");
    *command_writer = commands.to_vec();

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        cmd1.unwrap_or((ActuatorType::Vibrate, 0)).1 as u8,
        0x00,
        0x00,
        cmd2.unwrap_or((ActuatorType::Rotate, 0)).1 as u8,
        0xaa,
      ],
      false,
    )
    .into()])
  }
}
