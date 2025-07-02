// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
use std::sync::Arc;
use uuid::{uuid, Uuid};

const NOBRA_PROTOCOL_UUID: Uuid = uuid!("166e7d2b-b9ed-4769-aaaf-66127e4e14eb");
generic_protocol_initializer_setup!(Nobra, "nobra");

#[derive(Default)]
pub struct NobraInitializer {}

#[async_trait]
impl ProtocolInitializer for NobraInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[NOBRA_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x70],
        false,
      ))
      .await?;
    Ok(Arc::new(Nobra::default()))
  }
}

#[derive(Default)]
pub struct Nobra {}

impl ProtocolHandler for Nobra {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let output_speed = if speed == 0 { 0x70 } else { 0x60 + speed };
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![output_speed as u8],
      false,
    )
    .into()])
  }
}
