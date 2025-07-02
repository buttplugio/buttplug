// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
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

const LIONESS_PROTOCOL_UUID: Uuid = uuid!("1912c626-f611-4569-9d62-fb40ff8e1474");
generic_protocol_initializer_setup!(Lioness, "lioness");

#[derive(Default)]
pub struct LionessInitializer {}

#[async_trait]
impl ProtocolInitializer for LionessInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        LIONESS_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    let res = hardware
      .write_value(&HardwareWriteCmd::new(
        &[LIONESS_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x01, 0xAA, 0xAA, 0xBB, 0xCC, 0x10],
        true,
      ))
      .await;

    if res.is_err() {
      return Err(ButtplugDeviceError::DeviceCommunicationError(
        "Lioness may need pairing with OS. Use PIN 6496 or 006496 when pairing.".to_string(),
      ));
    }
    Ok(Arc::new(Lioness::default()))
  }
}

#[derive(Default)]
pub struct Lioness {}

impl ProtocolHandler for Lioness {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0x02, 0xAA, 0xBB, 0xCC, 0xCC, speed as u8],
      false,
    )
    .into()])
  }
}
