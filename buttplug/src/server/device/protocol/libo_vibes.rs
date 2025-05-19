// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{uuid, Uuid};

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{ActuatorType, Endpoint},
  },
  server::{device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_value_cmd::CheckedValueCmdV4},
};

const LIBO_VIBES_PROTOCOL_UUID: Uuid = uuid!("72a3d029-cf33-4fff-beec-1c45b85cc8ae");
generic_protocol_setup!(LiboVibes, "libo-vibes");

#[derive(Default)]
pub struct LiboVibes {}

impl ProtocolHandler for LiboVibes {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
  }

  fn handle_value_vibrate_cmd(
    &self,
    cmd: &CheckedValueCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    if cmd.feature_index() == 0 {
      msg_vec.push(HardwareWriteCmd::new(LIBO_VIBES_PROTOCOL_UUID, Endpoint::Tx, vec![cmd.value() as u8], false).into());
      // If this is a single vibe device, we need to send stop to TxMode too
      if cmd.value() as u8 == 0 {
        msg_vec.push(HardwareWriteCmd::new(LIBO_VIBES_PROTOCOL_UUID, Endpoint::TxMode, vec![0u8], false).into());
      }
    } else if cmd.feature_index() == 1 {
      msg_vec.push(HardwareWriteCmd::new(LIBO_VIBES_PROTOCOL_UUID, Endpoint::TxMode, vec![cmd.value() as u8], false).into());
    }
    Ok(msg_vec)
  }
}
