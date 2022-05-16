// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  generic_command_manager::GenericCommandManager,
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolFactory,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, Endpoint},
  server::device::{
    protocol::ButtplugProtocolProperties, 
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::{Hardware, HardwareWriteCmd}, 
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(TCodeV03, "tcode-v03");

impl ButtplugProtocolCommandHandler for TCodeV03 {
  fn handle_linear_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      let mut fut_vec = vec![];
      for v in msg.vectors() {
        let position = (v.position * 99f64) as u32;

        let command = format!("L{}{:02}I{}\n", v.index, position, v.duration);
        fut_vec.push(device.write_value(HardwareWriteCmd::new(
          Endpoint::Tx,
          command.as_bytes().to_vec(),
          false,
        )));
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some(speed) = cmd {
            let tcode_vibrate_cmd = format!("V{}{:02}\n", i, speed).as_bytes().to_vec();
            fut_vec.push(device.write_value(HardwareWriteCmd::new(
              Endpoint::Tx,
              tcode_vibrate_cmd,
              false,
            )));
          }
        }
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}
