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

const KIIROO_V2_PROTOCOL_UUID: Uuid = uuid!("05ab9d57-5e65-47b2-add4-5bad3e8663e5");
generic_protocol_initializer_setup!(KiirooV2, "kiiroo-v2");

#[derive(Default)]
pub struct KiirooV2Initializer {}

#[async_trait]
impl ProtocolInitializer for KiirooV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[KIIROO_V2_PROTOCOL_UUID],
      Endpoint::Firmware,
      vec![0x0u8],
      true,
    );
    hardware.write_value(&msg).await?;
    Ok(Arc::new(KiirooV2::default()))
  }
}

#[derive(Default)]
pub struct KiirooV2 {
  previous_position: Arc<AtomicU8>,
}

impl ProtocolHandler for KiirooV2 {
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
    let position = position as u8;
    let calculated_speed = (calculate_speed(distance, duration) * 99f64) as u8;
    self.previous_position.store(position, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      [position, calculated_speed].to_vec(),
      false,
    )
    .into()])
  }
}
