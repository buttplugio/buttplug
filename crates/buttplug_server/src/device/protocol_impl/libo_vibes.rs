// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

const LIBO_VIBES_PROTOCOL_UUID: Uuid = uuid!("72a3d029-cf33-4fff-beec-1c45b85cc8ae");
generic_protocol_setup!(LiboVibes, "libo-vibes");

#[derive(Default)]
pub struct LiboVibes {}

impl ProtocolHandler for LiboVibes {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    if feature_index == 0 {
      msg_vec.push(
        HardwareWriteCmd::new(
          &[LIBO_VIBES_PROTOCOL_UUID],
          Endpoint::Tx,
          vec![speed as u8],
          false,
        )
        .into(),
      );
      // If this is a single vibe device, we need to send stop to TxMode too
      if speed as u8 == 0 {
        msg_vec.push(
          HardwareWriteCmd::new(
            &[LIBO_VIBES_PROTOCOL_UUID],
            Endpoint::TxMode,
            vec![0u8],
            false,
          )
          .into(),
        );
      }
    } else if feature_index == 1 {
      msg_vec.push(
        HardwareWriteCmd::new(
          &[LIBO_VIBES_PROTOCOL_UUID],
          Endpoint::TxMode,
          vec![speed as u8],
          false,
        )
        .into(),
      );
    }
    Ok(msg_vec)
  }
}
