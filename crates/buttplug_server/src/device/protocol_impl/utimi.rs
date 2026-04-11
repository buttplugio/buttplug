// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::hardware::{HardwareCommand, HardwareWriteCmd};
use crate::device::{
  hardware::Hardware,
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use uuid::{Uuid, uuid};

const UTIMI_PROTOCOL_UUID: Uuid = uuid!("d4a3e2b1-7c56-4f89-a012-3b4c5d6e7f80");

generic_protocol_initializer_setup!(Utimi, "utimi");

#[derive(Default)]
pub struct UtimiInitializer {}

#[async_trait]
impl ProtocolInitializer for UtimiInitializer {
  async fn initialize(
    &mut self,
    _hardware: Arc<Hardware>,
    _def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(Utimi::default()))
  }
}

pub struct Utimi {
  // Benben lightspot protocol supports up to 5 motor slots.
  // Utimi devices use slot 0 (vibrate) and slot 1 (thrust/oscillate).
  last_cmds: [AtomicU8; 5],
}

impl Default for Utimi {
  fn default() -> Self {
    Self {
      last_cmds: [const { AtomicU8::new(0) }; 5],
    }
  }
}

impl Utimi {
  fn send_command(
    &self,
    index: u32,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_cmds[index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[UTIMI_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![
          0xa0,
          0x03,
          self.last_cmds[0].load(Ordering::Relaxed),
          self.last_cmds[1].load(Ordering::Relaxed),
          self.last_cmds[2].load(Ordering::Relaxed),
          self.last_cmds[3].load(Ordering::Relaxed),
          self.last_cmds[4].load(Ordering::Relaxed),
          0xaa,
        ],
        false,
      )
      .into(),
    ])
  }
}

impl ProtocolHandler for Utimi {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.send_command(feature_index, speed)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.send_command(feature_index, speed)
  }
}
