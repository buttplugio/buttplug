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

generic_protocol_initializer_setup!(LeloF1s, "lelo-f1s");

#[derive(Default)]
pub struct LeloF1sInitializer {}

#[async_trait]
impl ProtocolInitializer for LeloF1sInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // The Lelo F1s needs you to hit the power button after connection
    // before it'll accept any commands. Unless we listen for event on
    // the button, this is more likely to turn the device off.
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;
    Ok(Arc::new(LeloF1s::default()))
  }
}

#[derive(Default)]
pub struct LeloF1s {}

impl ProtocolHandler for LeloF1s {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut cmd_vec = vec![0x1];
    for cmd in cmds.iter() {
      cmd_vec.push(cmd.expect("LeloF1s should always send all values").1 as u8);
    }
    Ok(vec![
      HardwareWriteCmd::new(Endpoint::Tx, cmd_vec, false).into()
    ])
  }
}
