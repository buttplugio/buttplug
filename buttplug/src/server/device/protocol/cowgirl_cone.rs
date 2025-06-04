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
  util::sleep,
};
use async_trait::async_trait;
use uuid::Uuid;
use std::{sync::Arc, time::Duration};

generic_protocol_initializer_setup!(CowgirlCone, "cowgirl-cone");

#[derive(Default)]
pub struct CowgirlConeInitializer {}

#[async_trait]
impl ProtocolInitializer for CowgirlConeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(
        Uuid::nil(),
        Endpoint::Tx,
        vec![0xaa, 0x56, 0x00, 0x00],
        false,
      ))
      .await?;
    sleep(Duration::from_millis(3000)).await;
    Ok(Arc::new(CowgirlCone::default()))
  }
}

#[derive(Default)]
pub struct CowgirlCone {}

impl ProtocolHandler for CowgirlCone {
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
      vec![0xf1, 0x01, cmd.value() as u8, 0x00],
      false,
    )
    .into()])
  }
}
