// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    configuration::{UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolCommunicationSpecifier,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  }},
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::sync::Arc;

const AMORELIE_JOY_PROTOCOL_UUID: Uuid = uuid!("0968017b-96f8-44ae-b113-39080dd7ed5f");

generic_protocol_initializer_setup!(AmorelieJoy, "amorelie-joy");

#[derive(Default)]
pub struct AmorelieJoyInitializer {}

#[async_trait]
impl ProtocolInitializer for AmorelieJoyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(AMORELIE_JOY_PROTOCOL_UUID, Endpoint::Tx, vec![0x03], false))
      .await?;
    Ok(Arc::new(AmorelieJoy::default()))
  }
}

#[derive(Default)]
pub struct AmorelieJoy {}

impl ProtocolHandler for AmorelieJoy {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_actuator_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      [
        0x01,         // static header
        0x01,         // pattern (1 = steady),
        speed as u8, // speed 0-100
      ]
      .to_vec(),
      false,
    )
    .into()])
  }
}
