// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(ButtplugPassthru, "buttplug-passthru");

impl ButtplugProtocolCommandHandler for ButtplugPassthru {
  fn handle_command(
    &self,
    device: Arc<DeviceImpl>,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      device
        .write_value(DeviceWriteCmd::new(
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
