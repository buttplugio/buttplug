// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

use async_trait::async_trait;
use uuid::{uuid, Uuid};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::OutputType,
};
use buttplug_server_device_config::{Endpoint, DeviceDefinition, UserDeviceIdentifier, ProtocolCommunicationSpecifier};

generic_protocol_initializer_setup!(WeVibe8Bit, "wevibe-8bit");

const WEVIBE8BIT_PROTOCOL_UUID: Uuid = uuid!("f5e48973-09e9-4063-8177-487f6292e2ed");

#[derive(Default)]
pub struct WeVibe8BitInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibe8BitInitializer {
  async fn initialize(
    &mut self,
    _hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let num_vibrators = def
      .features()
      .iter()
      .filter(|x| {
        x.output()
          .as_ref()
          .map_or(false, |x| x.contains_key(&OutputType::Vibrate))
      })
      .count() as u8;
    Ok(Arc::new(WeVibe8Bit::new(num_vibrators)))
  }
}

pub struct WeVibe8Bit {
  num_vibrators: u8,
  speeds: [AtomicU8; 2],
}

impl WeVibe8Bit {
  fn new(num_vibrators: u8) -> Self {
    Self {
      num_vibrators,
      speeds: [AtomicU8::default(), AtomicU8::default()],
    }
  }
}

impl ProtocolHandler for WeVibe8Bit {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let max_vibrators = if self.num_vibrators > 1 { 1 } else { 0 };
    let r_speed_int = self.speeds[0].load(Ordering::Relaxed);
    let r_speed_ext = self.speeds[max_vibrators].load(Ordering::Relaxed);
    let data = if r_speed_int == 0 && r_speed_ext == 0 {
      vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    } else {
      let status_byte: u8 =
        (if r_speed_ext == 0 { 0 } else { 2 }) | (if r_speed_int == 0 { 0 } else { 1 });
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_ext + 3,
        r_speed_int + 3,
        status_byte,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(
      &[WEVIBE8BIT_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      true,
    )
    .into()])
  }
}
