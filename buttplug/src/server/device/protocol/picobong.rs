// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::device_impl::{ButtplugDeviceResultFuture, Hardware, HardwareWriteCmd},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(Picobong, "picobong");

impl ButtplugProtocolCommandHandler for Picobong {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager
    let speed = (msg.speeds()[0].speed() * 10.0) as u8;
    let mode: u8 = if speed == 0 { 0xff } else { 0x01 };
    let msg = HardwareWriteCmd::new(Endpoint::Tx, [0x01, mode, speed].to_vec(), false);
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write tests for protocol
