// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
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
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

generic_protocol_initializer_setup!(MetaXSireV3, "metaxsire-v3");
#[derive(Default)]
pub struct MetaXSireV3Initializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireV3Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let feature_count = device_definition
      .features()
      .iter()
      .filter(|x| x.actuator().is_some())
      .count();
    Ok(Arc::new(MetaXSireV3::new(hardware, feature_count)))
  }
}

const METAXSIRE_COMMAND_DELAY_MS: u64 = 100;

async fn command_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  trace!("Entering metaXsire v3 Control Loop");
  let mut current_commands = command_holder.read().await.clone();
  let mut errored = false;
  while !errored {
    for i in 0..current_commands.len() {
      if current_commands[i] == 0 {
        continue;
      }
      errored = !device
        .write_value(&HardwareWriteCmd::new(
          Endpoint::Tx,
          vec![0xa1, 0x04, current_commands[i], i as u8 + 1],
          false,
        ))
        .await
        .is_ok();
      if errored {
        break;
      }
    }
    sleep(Duration::from_millis(METAXSIRE_COMMAND_DELAY_MS)).await;
    current_commands = command_holder.read().await.clone();
    trace!("metaXsire v3 Command: {:?}", current_commands);
  }
  trace!("metaXsire v3 control loop exiting, most likely due to device disconnection.");
}

pub struct MetaXSireV3 {
  current_commands: Arc<RwLock<Vec<u8>>>,
}

impl MetaXSireV3 {
  fn new(device: Arc<Hardware>, feature_count: usize) -> Self {
    let current_commands = Arc::new(RwLock::new(vec![0u8; feature_count]));
    let current_commands_clone = current_commands.clone();
    async_manager::spawn(
      async move { command_update_handler(device, current_commands_clone).await },
    );
    Self { current_commands }
  }
}

impl ProtocolHandler for MetaXSireV3 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmds = vec![];
    for i in 0..commands.len() {
      if let Some(cmd) = commands[i] {
        let current_commands = self.current_commands.clone();
        async_manager::spawn(async move {
          let write_mutex = current_commands.clone();
          let mut command_writer = write_mutex.write().await;
          command_writer[i] = cmd.1 as u8;
        });
        cmds.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![0xa1, 0x04, cmd.1 as u8, i as u8 + 1],
            true,
          )
          .into(),
        );
      }
    }
    Ok(cmds)
  }
}
