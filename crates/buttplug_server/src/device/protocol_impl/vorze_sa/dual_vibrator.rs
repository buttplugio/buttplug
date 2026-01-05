// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::{Uuid, uuid};

use super::VorzeDevice;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::ProtocolHandler,
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use std::sync::atomic::{AtomicU8, Ordering};

// Vorze Omorfi needs a unified protocol UUID since we update both outputs in the same packet.
const VORZE_OMORFI_PROTOCOL_UUID: Uuid = uuid!("edfc1138-a6f7-4e5c-8f84-fc775b6da9d9");

#[derive(Default)]
pub struct VorzeSADualVibrator {
  speeds: [AtomicU8; 2],
}

impl ProtocolHandler for VorzeSADualVibrator {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[VORZE_OMORFI_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![
          VorzeDevice::Omorfi as u8,
          self.speeds[0].load(Ordering::Relaxed),
          self.speeds[1].load(Ordering::Relaxed),
        ],
        true,
      )
      .into(),
    ])
  }
}
