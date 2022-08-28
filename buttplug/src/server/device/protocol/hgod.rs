// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{HardwareCommand, HardwareWriteCmd, Hardware},
    protocol::{generic_protocol_initializer_setup, ProtocolHandler, ProtocolIdentifier, ProtocolInitializer,},
    ServerDeviceIdentifier,
  },
  util::async_manager
};
use futures::FutureExt;
use async_trait::async_trait;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::{
  time::sleep,
  sync::RwLock
};

// Time between Hgod update commands, in milliseconds.
const HGOD_COMMAND_DELAY_MS: u64 = 100;

generic_protocol_initializer_setup!(Hgod, "hgod");

#[derive(Default)]
pub struct HgodInitializer {}

#[async_trait]
impl ProtocolInitializer for HgodInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Hgod::new(&hardware)))
  }
}

pub struct Hgod {
  hardware: Arc<Hardware>,
  current_command: Arc<RwLock<Vec<u8>>>,
  updater_running: Arc<AtomicBool>,
}

impl Hgod {
  fn new(hardware: &Arc<Hardware>) -> Self {
    Self {
      hardware: hardware.clone(),
      updater_running: Arc::new(AtomicBool::new(false)),
      current_command: Arc::new(RwLock::new(vec![0x55, 0x04, 0, 0, 0, 0])),
    }
  }
}

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Hgod Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(&HardwareWriteCmd::new(Endpoint::Tx, current_command, true))
    .await
    .is_ok()
  {
    sleep(Duration::from_millis(HGOD_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("Hgod Command: {:?}", current_command);
  }
  info!("Hgod control loop exiting, most likely due to device disconnection.");
}

impl ProtocolHandler for Hgod {
  fn handle_scalar_vibrate_cmd(
      &self,
      _index: u32,
      scalar: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_command = self.current_command.clone();
    let update_running = self.updater_running.clone();
    let hardware = self.hardware.clone();
    async_manager::spawn(async move {
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      let command: Vec<u8> = vec![0x55, 0x04, 0, 0, 0, scalar as u8];
      *command_writer = command;
      if !update_running.load(Ordering::SeqCst) {
        async_manager::spawn(
          async move { vibration_update_handler(hardware, current_command).await },
        );
        update_running.store(true, Ordering::SeqCst);
      }
    }.boxed());
    Ok(vec![])
  }
}

// TODO Write some tests!
//
// At least, once I figure out how to do that with the weird timing on this
// thing.
