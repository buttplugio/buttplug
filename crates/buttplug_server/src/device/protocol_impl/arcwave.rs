// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use std::sync::Arc;
use uuid::{Uuid, uuid};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use crate::generic_protocol_initializer_setup;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

generic_protocol_initializer_setup!(Arcwave, "arcwave");

const ARCWAVE_PROTOCOL_UUID: Uuid = uuid!("cc0beeee-3d8a-4c2e-930f-30f944f26f18");
#[derive(Default)]
pub struct ArcwaveInitializer {}

#[async_trait]
impl ProtocolInitializer for ArcwaveInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[ARCWAVE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x0f, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04],
        false,
      ))
      .await?;
    Ok(Arc::new(Arcwave::default()))
  }
}

#[derive(Default)]
pub struct Arcwave {}

impl ProtocolHandler for Arcwave {
  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0x0f,
          if speed == 0 { 0x00 } else { 0x1a },
          speed.unsigned_abs() as u8,
          0x00,
          0x00,
          0x00,
          0x00,
          if speed == 0 {
            0x00
          } else if speed >= 0 {
            0x01
          } else {
            0x02
          },
        ],
        false,
      )
      .into(),
    ])
  }
}
