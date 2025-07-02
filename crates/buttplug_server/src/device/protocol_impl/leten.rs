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
    ProtocolInitializer, ProtocolKeepaliveStrategy,
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
use std::time::Duration;
use uuid::{uuid, Uuid};

const LETEN_PROTOCOL_UUID: Uuid = uuid!("7d899f44-2676-4a00-9c68-0c800055ee2a");

generic_protocol_initializer_setup!(Leten, "leten");
#[derive(Default)]
pub struct LetenInitializer {}

#[async_trait]
impl ProtocolInitializer for LetenInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // There's a more complex auth flow that the app "sometimes" goes through where it
    // sends [0x04, 0x00] and waits for [0x01] on Rx before calling [0x04, 0x01]
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[LETEN_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x04, 0x01],
        true,
      ))
      .await?;
    // Sometimes sending this causes Rx to receive [0x0a]
    Ok(Arc::new(Leten::default()))
  }
}

const LETEN_COMMAND_DELAY_MS: u64 = 1000;

#[derive(Default)]
pub struct Leten {}

impl ProtocolHandler for Leten {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    // Leten keepalive is shorter
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_millis(
      LETEN_COMMAND_DELAY_MS,
    ))
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0x02, speed as u8],
      true,
    )
    .into()])
  }
}
