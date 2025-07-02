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

const WETOY_PROTOCOL_ID: Uuid = uuid!("9868762e-4203-4876-abf5-83c992e024b4");
generic_protocol_initializer_setup!(WeToy, "wetoy");

#[derive(Default)]
pub struct WeToyInitializer {}

#[async_trait]
impl ProtocolInitializer for WeToyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[WETOY_PROTOCOL_ID],
        Endpoint::Tx,
        vec![0x80, 0x03],
        true,
      ))
      .await?;
    Ok(Arc::new(WeToy::default()))
  }
}

#[derive(Default)]
pub struct WeToy {}

impl ProtocolHandler for WeToy {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      if speed == 0 {
        vec![0x80, 0x03]
      } else {
        vec![0xb2, speed as u8 - 1]
      },
      true,
    )
    .into()])
  }
}
