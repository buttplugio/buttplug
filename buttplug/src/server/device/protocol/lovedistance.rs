// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{device::{
    configuration::{ProtocolCommunicationSpecifier, UserDeviceDefinition, UserDeviceIdentifier},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
  }, message::checked_actuator_cmd::CheckedActuatorCmdV4},
};
use async_trait::async_trait;
use uuid::{uuid, Uuid};
use std::sync::Arc;

const LOVEDISTANCE_PROTOCOL_UUID: Uuid = uuid!("a5f50cd5-7985-438c-a5bc-f8ff72bc0117");
generic_protocol_initializer_setup!(LoveDistance, "lovedistance");

#[derive(Default)]
pub struct LoveDistanceInitializer {}

#[async_trait]
impl ProtocolInitializer for LoveDistanceInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(LOVEDISTANCE_PROTOCOL_UUID, Endpoint::Tx, vec![0xf3, 0, 0], false);
    hardware.write_value(&msg).await?;
    let msg = HardwareWriteCmd::new(LOVEDISTANCE_PROTOCOL_UUID, Endpoint::Tx, vec![0xf4, 1], false);
    hardware.write_value(&msg).await?;
    Ok(Arc::new(LoveDistance::default()))
  }
}

#[derive(Default)]
pub struct LoveDistance {}

impl ProtocolHandler for LoveDistance {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedActuatorCmdV4
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      cmd.feature_id(),
      Endpoint::Tx,
      vec![0xf3, 0x00, cmd.value() as u8],
      false,
    )
    .into()])
  }
}
