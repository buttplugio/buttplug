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

generic_protocol_initializer_setup!(Leten, "leten");
#[derive(Default)]
pub struct LetenInitializer {}

#[async_trait]
impl ProtocolInitializer for LetenInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // There's a more complex auth flow that the app "sometimes" goes through where it
    // sends [0x04, 0x00] and waits for [0x01] on Rx before calling [0x04, 0x01]
    hardware
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, vec![0x04, 0x01], true))
      .await?;
    // Sometimes sending this causes Rx to receive [0x0a]
    Ok(Arc::new(Leten::new(hardware)))
  }
}

const LETEN_COMMAND_DELAY_MS: u64 = 1000;

async fn command_update_handler(device: Arc<Hardware>, command_holder: Arc<AtomicU8>) {
  trace!("Entering Leten keep-alive loop");
  let mut current_command = command_holder.load(Ordering::Relaxed);
  while device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x02, current_command],
      true,
    ))
    .await
    .is_ok()
  {
    sleep(Duration::from_millis(LETEN_COMMAND_DELAY_MS)).await;
    current_command = command_holder.load(Ordering::Relaxed);
    trace!("Leten Command: {:?}", current_command);
  }
  trace!("Leten keep-alive loop exiting, most likely due to device disconnection.");
}

pub struct Leten {
  current_command: Arc<AtomicU8>,
}

impl Leten {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(AtomicU8::new(0));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { command_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for Leten {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    // Leten keepalive is shorter
    super::ProtocolKeepaliveStrategy::NoStrategy
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
      vec![0x02, scalar as u8],
      true,
    )
    .into()])
  }
}
