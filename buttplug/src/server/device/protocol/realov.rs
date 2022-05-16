// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::{
    ButtplugServerResultFuture,
    device::{
      protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
      configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
      hardware::{Hardware, HardwareWriteCmd},
    },
  }
};
use std::sync::Arc;

super::default_protocol_declaration!(Realov, "realov");

impl ButtplugProtocolCommandHandler for Realov {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // TODO Convert to using generic command manager
    let speed = (msg.speeds()[0].speed() * 50.0) as u8;
    let msg = HardwareWriteCmd::new(Endpoint::Tx, [0xc5, 0x55, speed, 0xaa].to_vec(), false);
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write Tests
