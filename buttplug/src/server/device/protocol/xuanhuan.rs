// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::sync::RwLock;

generic_protocol_initializer_setup!(Xuanhuan, "xuanhuan");

#[derive(Default)]
pub struct XuanhuanInitializer {}

#[async_trait]
impl ProtocolInitializer for XuanhuanInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Xuanhuan::new(hardware)))
  }
}

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Xuanhuan Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while current_command == vec![0x03, 0x02, 0x00, 0x00]
    || device
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, current_command, true))
      .await
      .is_ok()
  {
    sleep(Duration::from_millis(300)).await;
    current_command = command_holder.read().await.clone();
  }
  info!("Xuanhuan control loop exiting, most likely due to device disconnection.");
}

pub struct Xuanhuan {
  current_command: Arc<RwLock<Vec<u8>>>,
}

impl Xuanhuan {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(RwLock::new(vec![0x03, 0x02, 0x00, 0x00]));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { vibration_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for Xuanhuan {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_command = self.current_command.clone();
    async_manager::spawn(async move {
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      *command_writer = vec![0x03, 0x02, 0x00, scalar as u8];
    });
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x03, 0x02, 0x00, scalar as u8],
      true,
    )
    .into()])
  }
}
