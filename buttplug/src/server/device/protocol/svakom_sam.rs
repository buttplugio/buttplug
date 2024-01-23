// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::{ProtocolAttributesType, ProtocolDeviceAttributes},
    hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
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
    let mut gen2 = hardware.endpoints().contains(&Endpoint::TxMode);
    if !gen2 && hardware.endpoints().contains(&Endpoint::Firmware) {
      gen2 = true;
      warn!("Svakom Sam model without speed control detected - This device will only vibrate at 1 speed");
    }

    Ok(Arc::new(SvakomSam::new(gen2)))
  }
}

pub struct SvakomSam {
  gen2: bool,
}
impl SvakomSam {
  pub fn new(gen2: bool) -> Self {
    Self { gen2 }
  }
}

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
        HardwareWriteCmd::new(
          Endpoint::Tx,
          if self.gen2 {
            [
              18,
              1,
              3,
              0,
              if speed == 0 { 0x00 } else { 0x04 },
              speed as u8,
            ]
            .to_vec()
          } else {
            [18, 1, 3, 0, 5, speed as u8].to_vec()
          },
          false,
        )
        .into(),
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
