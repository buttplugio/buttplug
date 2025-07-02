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
    ProtocolInitializer, ProtocolKeepaliveStrategy,
  },
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use std::{
  sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
  },
  time::Duration,
};
use uuid::{uuid, Uuid};

generic_protocol_initializer_setup!(MysteryVibe, "mysteryvibe");

const MYSTERYVIBE_PROTOCOL_UUID: Uuid = uuid!("53bca658-2efe-4388-8ced-333789bac20b");

#[derive(Default)]
pub struct MysteryVibeInitializer {}

#[async_trait]
impl ProtocolInitializer for MysteryVibeInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let msg = HardwareWriteCmd::new(
      &[MYSTERYVIBE_PROTOCOL_UUID],
      Endpoint::TxMode,
      vec![0x43u8, 0x02u8, 0x00u8],
      true,
    );
    hardware.write_value(&msg).await?;
    let vibrator_count = def
      .features()
      .iter()
      .filter(|x| x.output().is_some())
      .count();
    Ok(Arc::new(MysteryVibe::new(vibrator_count as u8)))
  }
}

// Time between Mysteryvibe update commands, in milliseconds. This is basically
// a best guess derived from watching packet timing a few years ago.
//
// Thelemic vibrator. Neat.
//
const MYSTERYVIBE_COMMAND_DELAY_MS: u64 = 93;

#[derive(Default)]
pub struct MysteryVibe {
  speeds: Vec<AtomicU8>,
}

impl MysteryVibe {
  pub fn new(vibrator_count: u8) -> Self {
    Self {
      speeds: std::iter::repeat_with(|| AtomicU8::default())
        .take(vibrator_count as usize)
        .collect(),
    }
  }
}

impl ProtocolHandler for MysteryVibe {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_millis(
      MYSTERYVIBE_COMMAND_DELAY_MS,
    ))
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[MYSTERYVIBE_PROTOCOL_UUID],
      Endpoint::TxVibrate,
      self
        .speeds
        .iter()
        .map(|x| x.load(Ordering::Relaxed))
        .collect(),
      false,
    )
    .into()])
  }
}
