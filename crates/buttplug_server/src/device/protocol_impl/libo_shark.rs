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

const LIBO_SHARK_PROTOCOL_UUID: Uuid = uuid!("c0044425-b59c-4037-a702-0438afcaad3e");
generic_protocol_setup!(LiboShark, "libo-shark");

#[derive(Default)]
pub struct LiboShark {
  values: [AtomicU8; 2],
}

impl ProtocolHandler for LiboShark {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.values[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let data = self.values[0].load(Ordering::Relaxed) << 4 | self.values[1].load(Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[LIBO_SHARK_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![data],
      false,
    )
    .into()])
  }
}
