// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::hardware::Hardware;
use crate::device::protocol::ProtocolInitializer;
use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_initializer_setup, ProtocolHandler, ProtocolIdentifier},
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

const METAXSIRE_V2_PROTOCOL_ID: Uuid = uuid!("28b934b4-ca45-4e14-85e7-4c1524b2b4c1");
generic_protocol_initializer_setup!(MetaXSireV2, "metaxsire-v2");

#[derive(Default)]
pub struct MetaXSireV2Initializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[METAXSIRE_V2_PROTOCOL_ID],
        Endpoint::Tx,
        vec![0xaa, 0x04],
        true,
      ))
      .await?;
    Ok(Arc::new(MetaXSireV2::default()))
  }
}

#[derive(Default)]
pub struct MetaXSireV2 {}

impl MetaXSireV2 {
  fn form_command(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0xaa,
        0x03,
        0x01,
        (feature_index + 1) as u8,
        0x64,
        speed as u8,
      ],
      true,
    )
    .into()])
  }
}

impl ProtocolHandler for MetaXSireV2 {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, feature_id, speed)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_command(feature_index, feature_id, speed)
  }
}
