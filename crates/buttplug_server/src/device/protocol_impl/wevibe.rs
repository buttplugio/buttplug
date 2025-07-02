// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};
use async_trait::async_trait;
use buttplug_core::message::OutputType;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use uuid::{uuid, Uuid};

const WEVIBE_PROTOCOL_UUID: Uuid = uuid!("3658e33d-086d-401e-9dce-8e9e88ff791f");
generic_protocol_initializer_setup!(WeVibe, "wevibe");

#[derive(Default)]
pub struct WeVibeInitializer {}

#[async_trait]
impl ProtocolInitializer for WeVibeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    debug!("calling WeVibe init");
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[WEVIBE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x0f, 0x03, 0x00, 0x99, 0x00, 0x03, 0x00, 0x00],
        true,
      ))
      .await?;
    hardware
      .write_value(&HardwareWriteCmd::new(
        &[WEVIBE_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        true,
      ))
      .await?;
    let num_vibrators = def
      .features()
      .iter()
      .filter(|x| {
        x.output()
          .as_ref()
          .map_or(false, |x| x.contains_key(&OutputType::Vibrate))
      })
      .count() as u8;
    Ok(Arc::new(WeVibe::new(num_vibrators)))
  }
}

pub struct WeVibe {
  num_vibrators: u8,
  speeds: [AtomicU8; 2],
}

impl WeVibe {
  fn new(num_vibrators: u8) -> Self {
    Self {
      num_vibrators,
      speeds: [AtomicU8::default(), AtomicU8::default()],
    }
  }
}

impl ProtocolHandler for WeVibe {
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
      vec![
        0x0f,
        0x03,
        0x00,
        r_speed_ext | (r_speed_int << 4),
        0x00,
        0x03,
        0x00,
        0x00,
      ]
    };
    Ok(vec![HardwareWriteCmd::new(
      &[WEVIBE_PROTOCOL_UUID],
      Endpoint::Tx,
      data,
      true,
    )
    .into()])
  }
}
