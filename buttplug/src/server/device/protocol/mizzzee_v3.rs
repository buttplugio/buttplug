// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
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
use std::sync::atomic::{AtomicU32, Ordering};
use std::{sync::Arc, time::Duration};

generic_protocol_initializer_setup!(MizzZeeV3, "mizzzee-v3");

#[derive(Default)]
pub struct MizzZeeV3Initializer {}

#[async_trait]
impl ProtocolInitializer for MizzZeeV3Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(MizzZeeV3::new(hardware)))
  }
}

// Time between MizzZee v3 update commands, in milliseconds.
const MIZZZEE3_COMMAND_DELAY_MS: u64 = 200;

fn handle_scale(scale: f32) -> f32 {
  if scale == 0.0 {
    return 0.0;
  }
  scale * 0.7 + 0.3
}

fn scalar_to_vector(scalar: u32) -> Vec<u8> {
  if scalar == 0 {
    return vec![
      0x03, 0x12, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
      0x00, 0x00, 0x00, 0x00, 0x00,
    ];
  }

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

async fn vibration_update_handler(device: Arc<Hardware>, current_scalar_holder: Arc<AtomicU32>) {
  info!("Entering Mizz Zee v3 Control Loop");
  let mut current_scalar = current_scalar_holder.load(Ordering::Relaxed);
  while device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      scalar_to_vector(current_scalar),
      true,
    ))
    .await
    .is_ok()
  {
    sleep(Duration::from_millis(MIZZZEE3_COMMAND_DELAY_MS)).await;
    current_scalar = current_scalar_holder.load(Ordering::Relaxed);
    trace!("Mizz Zee v3 scalar: {}", current_scalar);
  }
  info!("Mizz Zee v3 control loop exiting, most likely due to device disconnection.");
}

#[derive(Default)]
pub struct MizzZeeV3 {
  current_scalar: Arc<AtomicU32>,
}

impl MizzZeeV3 {
  fn new(device: Arc<Hardware>) -> Self {
    let current_scalar = Arc::new(AtomicU32::new(0));
    let current_scalar_clone = current_scalar.clone();
    async_manager::spawn(
      async move { vibration_update_handler(device, current_scalar_clone).await },
    );
    Self { current_scalar }
  }
}

impl ProtocolHandler for MizzZeeV3 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::NoStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let current_scalar = self.current_scalar.clone();
    current_scalar.store(scalar, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      scalar_to_vector(scalar),
      true,
    )
    .into()])
  }
}
