// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use uuid::{Uuid, uuid};

use buttplug_core::{errors::ButtplugDeviceError, message::OutputType};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    ProtocolKeepaliveStrategy,
    generic_protocol_initializer_setup,
  },
};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};

const SVAKOM_V4_VIBRATOR_UUID: Uuid = uuid!("0791b395-457a-4c1a-aa19-9d1a6116ebcc");

generic_protocol_initializer_setup!(SvakomV4, "svakom-v4");

#[derive(Default)]
pub struct SvakomV4Initializer {}

#[async_trait]
impl ProtocolInitializer for SvakomV4Initializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let num_vibrators = def
      .features()
      .values()
      .filter(|x| {
        if let Some(output_map) = x.output() {
          output_map.contains(OutputType::Vibrate)
        } else {
          false
        }
      })
      .count() as u8;
    Ok(Arc::new(SvakomV4::new(num_vibrators)))
  }
}

#[derive(Default)]
pub struct SvakomV4 {
  num_vibrators: u8,
  last_vibrator_speeds: [AtomicU8; 3],
}

impl SvakomV4 {
  fn new(num_vibrators: u8) -> Self {
    Self {
      num_vibrators,
      ..Default::default()
    }
  }
}

impl ProtocolHandler for SvakomV4 {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_vibrator_speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    let vibe1 = self.last_vibrator_speeds[0].load(Ordering::Relaxed);
    let vibe2 = self.last_vibrator_speeds[1].load(Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[SVAKOM_V4_VIBRATOR_UUID],
        Endpoint::Tx,
        [
          0x55,
          0x03,
          if self.num_vibrators == 1 || (vibe1 > 0 && vibe2 > 0) || vibe1 == vibe2 {
            0x00
          } else if vibe1 > 0 {
            0x01
          } else {
            0x02
          },
          0x00,
          if vibe1 == vibe2 && vibe1 == 0 {
            0x00
          } else {
            0x01
          },
          { vibe1.max(vibe2) },
          0x00,
        ]
        .to_vec(),
        false,
      )
      .into(),
    ])
  }
}
