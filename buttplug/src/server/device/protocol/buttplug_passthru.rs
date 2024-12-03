// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ButtplugClientMessageV4, Endpoint},
  },
  server::{
    device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
      protocol::{generic_protocol_setup, ProtocolHandler},
    },
    message::spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};

generic_protocol_setup!(ButtplugPassthru, "buttplug-passthru");

#[derive(Default)]
struct ButtplugPassthru {}

impl ProtocolHandler for ButtplugPassthru {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn has_handle_message(&self) -> bool {
    true
  }

  fn handle_message(
    &self,
    command_message: &ButtplugDeviceCommandMessageUnionV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      serde_json::to_string(&ButtplugClientMessageV4::from(command_message.clone()))
        .expect("Type is always serializable")
        .as_bytes()
        .to_vec(),
      false,
    )
    .into()])
  }
}
