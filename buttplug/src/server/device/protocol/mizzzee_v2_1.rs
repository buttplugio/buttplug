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
use std::sync::atomic::{AtomicU32, AtomicU8, Ordering};
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
const MIZZZEE2_COMMAND_DELAY_MS: u64 = 20;

// Time between MizzZee2 keep vibrating commands, in milliseconds.
const MIZZZEE2_COMMANDS_KEEP_VIBRATING: u64 = 200;

// Amount of commands that can be skipped without stopping the device.
const MIZZZEE2_COMMANDS_TO_SKIP: u8 = (MIZZZEE2_COMMANDS_KEEP_VIBRATING / MIZZZEE2_COMMAND_DELAY_MS - 1) as u8;

fn handle_scale(scale: f32) -> f32 {
  if scale == 0.0 { return 0.0; }
  scale * 0.7 + 0.3
}

fn scalar_to_vector(scalar: u32) -> Vec<u8> {
  const HEADER: [u8; 3] = [0x03, 0x12, 0xf3];
  const FILL_VEC: [u8; 6] = [0x00, 0xfc, 0x00, 0xfe, 0x40, 0x01];

  let scale: f32 = handle_scale(scalar as f32 / 1000.0) * 1023.0;
  let modded_scale: u16 = ((scale as u16) << 6) | 60;
  
  let bytes = modded_scale.swap_bytes().to_be_bytes();

  let mut data: Vec<u8> = Vec::new();
  data.extend_from_slice(&HEADER);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&bytes);
  data.extend_from_slice(&FILL_VEC);
  data.extend_from_slice(&bytes);
  data.push(0x00);

  data
}

async fn vibration_update_handler(
  device: Arc<Hardware>,
  loops_skipped_holder: Arc<AtomicU8>,
  last_scalar_holder: Arc<AtomicU32>,
  current_scalar_holder: Arc<AtomicU32>,
) {
  info!("Entering MizzZee2 Control Loop");
  loop {
    sleep(Duration::from_millis(MIZZZEE2_COMMAND_DELAY_MS)).await;

    let loops_skipped = loops_skipped_holder.load(Ordering::Relaxed);
    let last_scalar = last_scalar_holder.load(Ordering::Relaxed);
    let current_scalar = current_scalar_holder.load(Ordering::Relaxed);

    if last_scalar == current_scalar && loops_skipped < MIZZZEE2_COMMANDS_TO_SKIP {
      loops_skipped_holder.store(loops_skipped + 1, Ordering::Relaxed);
      continue;
    }

    loops_skipped_holder.store(0, Ordering::Relaxed);
    last_scalar_holder.store(current_scalar, Ordering::Relaxed);

    if device
      .write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        scalar_to_vector(current_scalar),
        true
      ))
      .await
      .is_err() { break; }

    info!("MZ2 scalar: {}", current_scalar);
  }
  info!("MizzZee2 control loop exiting, most likely due to device disconnection.");
}

#[derive(Default)]
pub struct MizzZeeV2_1 {
  current_scalar: Arc<AtomicU32>,
}

impl MizzZeeV2_1 {
  fn new(device: Arc<Hardware>) -> Self {
    let loops_skipped = Arc::new(AtomicU8::new(0));
    let loops_skipped_clone = loops_skipped.clone();

    let last_scalar = Arc::new(AtomicU32::new(0));
    let last_scalar_clone = last_scalar.clone();

    let current_scalar = Arc::new(AtomicU32::new(0));
    let current_scalar_clone = current_scalar.clone();

    async_manager::spawn( async move {
      vibration_update_handler(device, loops_skipped_clone, last_scalar_clone, current_scalar_clone).await
    });
    Self { current_scalar }
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
    let current_scalar = self.current_scalar.clone();
    async_manager::spawn(async move {
      current_scalar.store(scalar, Ordering::Relaxed);
    });
    Ok(vec![])
  }
}
