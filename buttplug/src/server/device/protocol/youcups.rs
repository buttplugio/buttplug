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
    device::device_impl::{ButtplugDeviceResultFuture, DeviceImpl, DeviceWriteCmd},
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(Youcups, "youcups");

impl ButtplugProtocolCommandHandler for Youcups {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager
    let msg = DeviceWriteCmd::new(
      Endpoint::Tx,
      format!("$SYS,{}?", (msg.speeds()[0].speed() * 8.0) as u8)
        .as_bytes()
        .to_vec(),
      false,
    );
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write Tests
