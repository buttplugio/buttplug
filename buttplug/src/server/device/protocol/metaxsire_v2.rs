// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::ActuatorType;
use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::server::device::hardware::Hardware;
use crate::server::device::protocol::ProtocolInitializer;
use crate::{
  core::{errors::ButtplugDeviceError, message::Endpoint},
  server::{
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::{
        generic_protocol_initializer_setup,
        ProtocolAttributesType,
        ProtocolHandler,
        ProtocolIdentifier,
      },
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(MetaXSireV2, "metaxsire-v2");

#[derive(Default)]
pub struct MetaXSireV2Initializer {}

#[async_trait]
impl ProtocolInitializer for MetaXSireV2Initializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .write_value(&HardwareWriteCmd::new(Endpoint::Tx, vec![0xaa, 0x04], true))
      .await?;
    Ok(Arc::new(MetaXSireV2::default()))
  }
}

#[derive(Default)]
pub struct MetaXSireV2 {}

impl ProtocolHandler for MetaXSireV2 {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut hcmds = vec![];
    for i in 0..commands.len() {
      if let Some(cmd) = commands[i] {
        hcmds.push(
          HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![0xaa, 0x03, 0x01, (i + 1) as u8, 0x64, cmd.1 as u8],
            true,
          )
          .into(),
        );
      }
    }

    Ok(hcmds)
  }
}
