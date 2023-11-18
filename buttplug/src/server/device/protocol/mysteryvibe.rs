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
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

generic_protocol_initializer_setup!(MysteryVibe, "mysteryvibe");

#[derive(Default)]
pub struct MysteryVibeInitializer {}

#[async_trait]
impl ProtocolInitializer for MysteryVibeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(Endpoint::TxMode, vec![0x43u8, 0x02u8, 0x00u8], true);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(MysteryVibe::new(hardware)))
  }
}

// Time between Mysteryvibe update commands, in milliseconds. This is basically
// a best guess derived from watching packet timing a few years ago.
//
// Thelemic vibrator. Neat.
//
const MYSTERYVIBE_COMMAND_DELAY_MS: u64 = 93;

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Mysteryvibe Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::TxVibrate,
      current_command,
      false,
    ))
    .await
    .is_ok()
  {
    sleep(Duration::from_millis(MYSTERYVIBE_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("MV Command: {:?}", current_command);
  }
  info!("Mysteryvibe control loop exiting, most likely due to device disconnection.");
}

pub struct MysteryVibe {
  current_command: Arc<RwLock<Vec<u8>>>,
}

impl MysteryVibe {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(RwLock::new(vec![0u8, 0, 0, 0, 0, 0]));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { vibration_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for MysteryVibe {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_command = self.current_command.clone();
    let cmds = cmds.to_vec();
    async_manager::spawn(async move {
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      let command: Vec<u8> = cmds
        .into_iter()
        .map(|x| x.expect("Validity ensured via GCM match_all").1 as u8)
        .collect();
      *command_writer = command;
    });
    Ok(vec![])
  }
}

// TODO Write some tests!
//
// At least, once I figure out how to do that with the weird timing on this
// thing.
