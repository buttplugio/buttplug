// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::sync::Arc;
use uuid::{Uuid, uuid};

const MYMUSELINKPLUS_PROTOCOL_UUID: Uuid = uuid!("b8c3a1f0-7d2e-4a19-9f6b-3e8d1c5a2b40");
generic_protocol_initializer_setup!(MyMuseLinkPlus, "mymuselinkplus");

#[derive(Default)]
pub struct MyMuseLinkPlusInitializer {}

#[async_trait]
impl ProtocolInitializer for MyMuseLinkPlusInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // Short vibration pulse to indicate the device is active
    let on = HardwareWriteCmd::new(
      &[MYMUSELINKPLUS_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0xAA, 0x55, 0x06, 0x01, 0x01, 0x01, 0x01, 0xFF],
      false,
    );
    hardware.write_value(&on).await?;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    // Turn off
    let off = HardwareWriteCmd::new(
      &[MYMUSELINKPLUS_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0xAA, 0x55, 0x06, 0xAA, 0x00, 0x00, 0x00, 0x00],
      false,
    );
    hardware.write_value(&off).await?;
    Ok(Arc::new(MyMuseLinkPlus::default()))
  }
}

#[derive(Default)]
pub struct MyMuseLinkPlus {}

impl ProtocolHandler for MyMuseLinkPlus {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Map 10 slider steps to 3 device intensity levels: SLOW(1), MEDIUM(2), FAST(3)
    let mode = match speed {
      0 => 0,
      1..=3 => 1,   // SLOW
      4..=6 => 2,   // MEDIUM
      7..=10 => 3,  // FAST
      _ => 3,
    };
    let data = if mode == 0 {
      vec![0xAA, 0x55, 0x06, 0xAA, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![0xAA, 0x55, 0x06, 0x01, 0x01, 0x01, mode, 0xFF]
    };

    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, data, false).into(),
    ])
  }
}
