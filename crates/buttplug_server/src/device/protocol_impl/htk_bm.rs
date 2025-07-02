// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::atomic::{AtomicU8, Ordering};

use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

const HTK_BM_PROTOCOL_UUID: Uuid = uuid!("4c70cb95-d3d9-4288-81ab-be845f9ad1fe");
generic_protocol_setup!(HtkBm, "htk_bm");

pub struct HtkBm {
  speeds: [AtomicU8; 2],
}

impl Default for HtkBm {
  fn default() -> Self {
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for HtkBm {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    let mut data: u8 = 15;
    let left = self.speeds[0].load(Ordering::Relaxed);
    let right = self.speeds[1].load(Ordering::Relaxed);
    if left != 0 && right != 0 {
      data = 11 // both (normal mode)
    } else if left != 0 {
      data = 12 // left only
    } else if right != 0 {
      data = 13 // right only
    }
    Ok(vec![HardwareWriteCmd::new(
      &[HTK_BM_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![data],
      false,
    )
    .into()])
  }
}
