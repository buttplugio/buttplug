// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::UserDeviceDefinition,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolCommunicationSpecifier,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
      UserDeviceIdentifier,
    },
  },
  util::async_manager,
};
use async_trait::async_trait;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

generic_protocol_initializer_setup!(Luvmazer, "luvmazer");

async fn delayed_rotate_handler(device: Arc<Hardware>, scalar: u8) {
  sleep(Duration::from_millis(25)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xa0, 0x0f, 0x00, 0x00, 0x64, scalar as u8],
      false,
    ))
    .await;
  if res.is_err() {
    error!("Delayed Luvmazer Rotate command error: {:?}", res.err());
  }
}
#[derive(Default)]
pub struct LuvmazerInitializer {}

#[async_trait]
impl ProtocolInitializer for LuvmazerInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &UserDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Luvmazer::new(hardware)))
  }
}

pub struct Luvmazer {
  device: Arc<Hardware>,
}

impl Luvmazer {
  fn new(device: Arc<Hardware>) -> Self {
    Self { device }
  }
}

impl ProtocolHandler for Luvmazer {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xa0, 0x01, 0x00, 0x00, 0x64, scalar as u8],
      false,
    )
    .into()])
  }

  fn handle_scalar_rotate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0xa0, 0x0f, 0x00, 0x00, 0x64, scalar as u8],
      false,
    )
    .into()])
  }

  fn handle_scalar_cmd(
    &self,
    commands: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let cmd1 = commands[0];
    let cmd2 = if commands.len() > 1 {
      commands[1]
    } else {
      None
    };

    if let Some(cmd) = cmd2 {
      if cmd.0 == ActuatorType::Rotate {
        if cmd1.is_some() {
          let dev = self.device.clone();
          async_manager::spawn(async move { delayed_rotate_handler(dev, cmd.1 as u8).await });
        } else {
          return Ok(vec![HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![0xa0, 0x0f, 0x00, 0x00, 0x64, cmd.1 as u8],
            false,
          )
          .into()]);
        }
      }
    }

    if let Some(cmd) = cmd1 {
      return Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![0xa0, 0x01, 0x00, 0x00, 0x64, cmd.1 as u8],
        false,
      )
      .into()]);
    }

    Ok(vec![])
  }
}
