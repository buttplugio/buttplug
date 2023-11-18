// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use std::sync::Arc;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct VibratissimoIdentifierFactory {}

  impl ProtocolIdentifierFactory for VibratissimoIdentifierFactory {
    fn identifier(&self) -> &str {
      "vibratissimo"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::VibratissimoIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct VibratissimoIdentifier {}

#[async_trait]
impl ProtocolIdentifier for VibratissimoIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(Endpoint::RxBLEModel, 128, 500))
      .await?;
    let ident =
      String::from_utf8(result.data().to_vec()).unwrap_or_else(|_| hardware.name().to_owned());
    Ok((
      ServerDeviceIdentifier::new(
        hardware.address(),
        "vibratissimo",
        &ProtocolAttributesType::Identifier(ident),
      ),
      Box::new(VibratissimoInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct VibratissimoInitializer {}

#[async_trait]
impl ProtocolInitializer for VibratissimoInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Vibratissimo::default()))
  }
}

#[derive(Default)]
pub struct Vibratissimo {}

impl ProtocolHandler for Vibratissimo {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut data: Vec<u8> = Vec::new();
    for cmd in cmds {
      data.push(cmd.unwrap_or((ActuatorType::Vibrate, 0)).1 as u8);
    }
    if data.len() == 1 {
      data.push(0x00);
    }

    // Put the device in write mode
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::TxMode, vec![0x03, 0xff], false).into(),
      HardwareWriteCmd::new(Endpoint::TxVibrate, data, false).into(),
    ])
  }
}
