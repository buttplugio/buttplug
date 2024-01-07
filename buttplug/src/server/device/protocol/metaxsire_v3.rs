// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{
    device::{
      configuration::ProtocolDeviceAttributes,
      hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
      protocol::{
        generic_protocol_initializer_setup,
        ProtocolAttributesType,
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
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use std::time::Duration;

generic_protocol_initializer_setup!(MetaXSireV3, "metaxsire-v3");
#[derive(Default)]
pub struct MetaXSireV3Initializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireV3Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(MetaXSireV3::new(hardware)))
  }
}

const METAXSIRE_COMMAND_DELAY_MS: u64 = 100;

async fn command_update_handler(device: Arc<Hardware>, command_holder: Arc<AtomicU8>) {
  trace!("Entering metaXsire v3 Control Loop");
  let mut current_command = command_holder.load(Ordering::Relaxed);
  while current_command == 0
    || device
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0xa1, 0x04, current_command, 0x01],
        false,
      ))
      .await
      .is_ok()
  {
    sleep(Duration::from_millis(METAXSIRE_COMMAND_DELAY_MS)).await;
    current_command = command_holder.load(Ordering::Relaxed);
    trace!("metaXsire v3 Command: {:?}", current_command);
  }
  trace!("metaXsire v3 control loop exiting, most likely due to device disconnection.");
}

pub struct MetaXSireV3 {
  current_command: Arc<AtomicU8>,
}

impl MetaXSireV3 {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(AtomicU8::new(0));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { command_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for MetaXSireV3 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_command = self.current_command.clone();
    current_command.store(scalar as u8, Ordering::Relaxed);

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xa1, 0x04, scalar as u8, 0x01],
      true,
    )
    .into()])
  }
}
