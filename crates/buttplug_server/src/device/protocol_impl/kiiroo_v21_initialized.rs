// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::fleshlight_launch_helper::calculate_speed;

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use uuid::{uuid, Uuid};

const KIIROO_V21_INITIALIZED_PROTOCOL_UUID: Uuid = uuid!("22329023-5464-41b6-a0de-673d7e993055");

generic_protocol_initializer_setup!(KiirooV21Initialized, "kiiroo-v21-initialized");

#[derive(Default)]
pub struct KiirooV21InitializedInitializer {}

#[async_trait]
impl ProtocolInitializer for KiirooV21InitializedInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    debug!("calling Onyx+ init");
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[KIIROO_V21_INITIALIZED_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x03u8, 0x00u8, 0x64u8, 0x19u8],
        true,
      ))
      .await?;
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[KIIROO_V21_INITIALIZED_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x03u8, 0x00u8, 0x64u8, 0x00u8],
        true,
      ))
      .await?;
    Ok(Arc::new(KiirooV21Initialized::default()))
  }
}

#[derive(Default)]
pub struct KiirooV21Initialized {
  previous_position: Arc<AtomicU8>,
}

impl ProtocolHandler for KiirooV21Initialized {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0x01, speed as u8],
      false,
    )
    .into()])
  }

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(Ordering::Relaxed);
    let distance = (previous_position as f64 - (position as f64)).abs() / 99f64;
    let calculated_speed = (calculate_speed(distance, duration) * 99f64) as u8;

    self
      .previous_position
      .store(position as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [0x03, 0x00, calculated_speed, position as u8].to_vec(),
      false,
    )
    .into()])
  }
}
