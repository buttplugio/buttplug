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
use buttplug_server_device_config::{DeviceDefinition, Endpoint, ProtocolCommunicationSpecifier, UserDeviceIdentifier};

generic_protocol_initializer_setup!(WeVibeChorus, "wevibe-chorus");

const WEVIBE_CHORUS_PROTOCOL_UUID: Uuid = uuid!("cdeadd1c-b913-4305-a255-bd8834c4e37f");

#[derive(Default)]
pub struct WeVibeChorusInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibeChorusInitializer {
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
    Ok(Arc::new(WeVibeChorus::new(num_vibrators)))
  }
}

pub struct WeVibeChorus {
  num_vibrators: u8,
  speeds: [AtomicU8; 2],
}

impl WeVibeChorus {
  fn new(num_vibrators: u8) -> Self {
    Self {
      num_vibrators,
      speeds: [AtomicU8::default(), AtomicU8::default()],
    }
  }
}

impl ProtocolHandler for WeVibeChorus {
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
      // Note the motor order is flipped for the Chorus
      let status_byte: u8 =
        (if r_speed_ext == 0 { 0 } else { 2 }) | (if r_speed_int == 0 { 0 } else { 1 });
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_int,
        r_speed_ext,
        status_byte,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(
      &[WEVIBE_CHORUS_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      true,
    )
    .into()])
  }
}
