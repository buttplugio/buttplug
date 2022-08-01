// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    Endpoint
  },
  server::{
    ButtplugServerResultFuture,
    device::{
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
      hardware::{Hardware, HardwareWriteCmd},
    },
  },
  util::async_manager,
};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::{Mutex, RwLock};

// Time between Hgod update commands, in milliseconds.
const HGOD_COMMAND_DELAY_MS: u64 = 100;

pub struct Hgod {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  current_command: Arc<RwLock<Vec<u8>>>,
  updater_running: Arc<AtomicBool>,
}

crate::default_protocol_properties_definition!(Hgod);

impl Hgod {
  const PROTOCOL_IDENTIFIER: &'static str = "hgod";

  fn new(device_attributes: ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      updater_running: Arc::new(AtomicBool::new(false)),
      current_command: Arc::new(RwLock::new(vec![0x55, 0x04, 0, 0, 0, 0])),
    }
  }
}

impl ButtplugProtocol for Hgod {}

super::default_protocol_trait_declaration!(Hgod);

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Hgod Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(HardwareWriteCmd::new(Endpoint::Tx, current_command, true))
    .await
    .is_ok()
  {
    Delay::new(Duration::from_millis(HGOD_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("Hgod Command: {:?}", current_command);
  }
  info!("Hgod control loop exiting, most likely due to device disconnection.");
}

impl ProtocolHandler for Hgod {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let manager = self.manager.clone();
    let current_command = self.current_command.clone();
    let update_running = self.updater_running.clone();
    async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      info!("Hgod Result: {:?}", result);
      if result.is_none() {
        return Ok(messages::Ok::default().into());
      }
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          let write_mutex = current_command.clone();
          let mut command_writer = write_mutex.write().await;
          let command: Vec<u8> = vec![0x55, 0x04, 0, 0, 0, speed as u8];
          *command_writer = command;
          if !update_running.load(Ordering::SeqCst) {
            async_manager::spawn(
              async move { vibration_update_handler(device, current_command).await },
            );
            update_running.store(true, Ordering::SeqCst);
          }
        }
      }
      Ok(messages::Ok::default().into())
    }.boxed()
  }
}

// TODO Write some tests!
//
// At least, once I figure out how to do that with the weird timing on this
// thing.
