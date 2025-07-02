// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
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
use std::cmp::{max, min};
use std::sync::Arc;
use uuid::{uuid, Uuid};

const LOOB_PROTOCOL_UUID: Uuid = uuid!("b3a02457-3bda-4c5b-8363-aead6eda74ae");
generic_protocol_initializer_setup!(Loob, "loob");

#[derive(Default)]
pub struct LoobInitializer {}

#[async_trait]
impl ProtocolInitializer for LoobInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[LOOB_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0x00, 0x01, 0x01, 0xf4],
      true,
    );
    hardware.write_value(&msg).await?;
    Ok(Arc::new(Loob::default()))
  }
}

#[derive(Default)]
pub struct Loob {}

impl ProtocolHandler for Loob {
  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let pos: u16 = max(min(position as u16, 1000), 1);
    let time: u16 = max(duration as u16, 1);
    let mut data = pos.to_be_bytes().to_vec();
    for b in time.to_be_bytes() {
      data.push(b);
    }
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      data,
      false,
    )
    .into()])
  }
}
