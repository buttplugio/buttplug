// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::core::message::ActuatorType::{Constrict, Rotate, Vibrate};
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{
    device::{
      hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
      protocol::{
        generic_protocol_initializer_setup,
        ProtocolAttributesType,
        ProtocolDeviceAttributes,
        ProtocolHandler,
        ProtocolIdentifier,
        ProtocolInitializer,
      },
    },
    ServerDeviceIdentifier,
  },
  util::{async_manager, sleep},
};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

generic_protocol_initializer_setup!(MetaXSireRepeat, "metaxsire-repeat");
#[derive(Default)]
pub struct MetaXSireRepeatInitializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireRepeatInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(MetaXSireRepeat::new(hardware)))
  }
}

const METAXSIRE_COMMAND_DELAY_MS: u64 = 100;

async fn command_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering metaXsire Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while current_command[0] == 0
    || device
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, current_command, false))
      .await
      .is_ok()
  {
    sleep(Duration::from_millis(METAXSIRE_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    trace!("metaXsire Command: {:?}", current_command);
  }
  info!("metaXsire control loop exiting, most likely due to device disconnection.");
}

pub struct MetaXSireRepeat {
  current_command: Arc<RwLock<Vec<u8>>>,
}

impl MetaXSireRepeat {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(RwLock::new(vec![0u8]));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { command_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for MetaXSireRepeat {
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
    let current_command = self.current_command.clone();
    let commands = commands.to_vec();
    async_manager::spawn(async move {
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      let mut data: Vec<u8> = vec![0x23, 0x07];
      data.push((commands.len() * 3) as u8);

      for (i, item) in commands.iter().enumerate() {
        let cmd = item.unwrap_or((Vibrate, 0));
        // motor number
        data.push(0x80 | ((i + 1) as u8));
        // motor type: 03=vibe 04=pump 06=rotate
        data.push(if cmd.0 == Rotate {
          0x06
        } else if cmd.0 == Constrict {
          0x04
        } else {
          0x03
        });
        data.push(cmd.1 as u8);
      }

      let mut crc: u8 = 0;
      for b in data.clone() {
        crc ^= b;
      }
      data.push(crc);

      *command_writer = data;
    });
    Ok(vec![])
  }
}
