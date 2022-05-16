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

super::default_protocol_declaration!(ButtplugPassthru, "buttplug-passthru");

impl ButtplugProtocolCommandHandler for ButtplugPassthru {
  fn handle_command(
    &self,
    device: Arc<Hardware>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    Box::pin(async move {
      device
        .write_value(HardwareWriteCmd::new(
          Endpoint::Tx,
          serde_json::to_string(&command_message)
            .expect("Type is always serializable")
            .as_bytes()
            .to_vec(),
          false,
        ))
        .await?;
      Ok(messages::Ok::default().into())
    })
  }
}
