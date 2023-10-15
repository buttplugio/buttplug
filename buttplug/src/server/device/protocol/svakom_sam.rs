// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::hardware::HardwareSubscribeCmd;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{ProtocolAttributesType, ProtocolDeviceAttributes},
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

generic_protocol_initializer_setup!(SvakomSam, "svakom-sam");

#[derive(Default)]
pub struct SvakomSamInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomSamInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;
    Ok(Arc::new(SvakomSam::default()))
  }
}

#[derive(Default)]
pub struct SvakomSam {}

impl ProtocolHandler for SvakomSam {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    if let Some((_, speed)) = cmds[0] {
      msg_vec.push(
        HardwareWriteCmd::new(Endpoint::Tx, [18, 1, 3, 0, 5, speed as u8].to_vec(), false).into(),
      );
    }
    if cmds.len() > 1 {
      if let Some((_, speed)) = cmds[1] {
        msg_vec.push(
          HardwareWriteCmd::new(Endpoint::Tx, [18, 6, 1, speed as u8].to_vec(), false).into(),
        );
      }
    }
    Ok(msg_vec)
  }
}
