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
    message::Endpoint
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

generic_protocol_initializer_setup!(MizzZeeV2_1, "mizzzee-v2.1");

#[derive(Default)]
pub struct MizzZeeV2_1Initializer {}

#[async_trait]
impl ProtocolInitializer for MizzZeeV2_1Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(MizzZeeV2_1::new(hardware)))
  }
}

// Time between MizzZee2 update commands, in milliseconds.
const MIZZZEE2_COMMAND_DELAY_MS: u64 = 75;

fn handle_scale(scale: f32) -> f32 {
  if scale == 0.0 { return 0.0; }
  scale * 0.7 + 0.3
}

fn scalar_to_vector(scalar: u32) -> Vec<u8> {
  const HEADER: [u8; 3] = [0x03, 0x12, 0xf3];
  const FILL_VEC: [u8; 6] = [0x00, 0xfc, 0x00, 0xfe, 0x40, 0x01];

  let scale: f32 = handle_scale(scalar as f32 / 1000.0) * 1023.0;
  let modded_scale: u16 = ((scale as u16) << 6) | 60;

  let first_byte: u8 = (modded_scale >> 8) as u8;
  let second_byte: u8 = modded_scale as u8;

  let mut data: Vec<u8> = Vec::new();
  data.extend_from_slice(&HEADER);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&[second_byte, first_byte]);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&[second_byte, first_byte]);
  data.push(0x00);

  data
}

async fn vibration_update_handler(device: Arc<Hardware>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering MizzZee2 Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      current_command,
      true
    ))
    .await
    .is_ok()
  {
    sleep(Duration::from_millis(MIZZZEE2_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("MZ2 Command: {:?}", current_command);
  }
  info!("MizzZee2 control loop exiting, most likely due to device disconnection.");
}

#[derive(Default)]
pub struct MizzZeeV2_1 {
  current_command: Arc<RwLock<Vec<u8>>>
}

impl MizzZeeV2_1 {
  fn new(device: Arc<Hardware>) -> Self {
    let current_command = Arc::new(RwLock::new(vec![0u8, 0, 0, 0, 0, 0]));
    let current_command_clone = current_command.clone();
    async_manager::spawn(
      async move { vibration_update_handler(device, current_command_clone).await },
    );
    Self { current_command }
  }
}

impl ProtocolHandler for MizzZeeV2_1 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_command = self.current_command.clone();
    async_manager::spawn(async move {
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      *command_writer = scalar_to_vector(scalar);
    });
    Ok(vec![])
  }
}
