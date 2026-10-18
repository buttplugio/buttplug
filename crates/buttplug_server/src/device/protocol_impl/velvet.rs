// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Velvet Rituals toys (BLE names `VR-*`). Same firmware family as Cupido (`B0 03 … AA`), with
//! Velvet's own service and these frames (verified on hardware by Velvet, Oct 2026):
//!
//! - Motors: `B0 03 00 00 <motor2> <motor1> AA`. Single-motor toys get both bytes (the
//!   Velvet apps and SDK send A = B). Motor 1 is Eclipse's outer motor, motor 2 the insertable.
//! - Heat (Ember): `B0 04 <on> 00 <level 0–2> 00`, no `AA` trailer.
//! - Battery: write `B0 00`, the reply on rx is `B0 00 …` with the percentage at byte 5.
//! - Eclipse ignores a change of *which* motor runs while one is running: the firmware needs the
//!   zero frame and a pause (300 ms on hardware) before the new frame. Strength changes on the same
//!   motor(s) apply at once.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use uuid::{Uuid, uuid};

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::message::{InputReadingV4, InputTypeReading, InputValue, OutputType};
use buttplug_core::util::async_manager;
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};

const VELVET_PROTOCOL_UUID: Uuid = uuid!("6c2f1c6e-3d53-4b2e-9c5a-5d0a6f2a7e11");
/// Pause between the zero frame and the new frame when Eclipse switches motors.
const MOTOR_SWITCH_SETTLE: Duration = Duration::from_millis(300);

generic_protocol_initializer_setup!(Velvet, "velvet");

#[derive(Default)]
pub struct VelvetInitializer {}

#[async_trait]
impl ProtocolInitializer for VelvetInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    def: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let dual = def
      .features()
      .values()
      .filter(|f| f.contains_output(OutputType::Vibrate))
      .count()
      >= 2;
    Ok(Arc::new(Velvet::new(hardware, dual)))
  }
}

pub struct Velvet {
  hardware: Arc<Hardware>,
  dual: bool,
  /// Last levels sent: [motor1, motor2].
  motors: [AtomicU8; 2],
  /// Bumped on every motor command, so a delayed post-switch frame can tell it's stale.
  generation: Arc<AtomicU32>,
}

fn motor_frame(motor1: u8, motor2: u8) -> Vec<u8> {
  vec![0xb0, 0x03, 0x00, 0x00, motor2, motor1, 0xaa]
}

impl Velvet {
  fn new(hardware: Arc<Hardware>, dual: bool) -> Self {
    Self {
      hardware,
      dual,
      motors: [const { AtomicU8::new(0) }; 2],
      generation: Arc::new(AtomicU32::new(0)),
    }
  }

  fn write(frame: Vec<u8>) -> HardwareCommand {
    HardwareWriteCmd::new(&[VELVET_PROTOCOL_UUID], Endpoint::Tx, frame, false).into()
  }
}

impl ProtocolHandler for Velvet {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let level = speed.min(255) as u8;
    let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

    if !self.dual {
      self.motors[0].store(level, Ordering::Relaxed);
      return Ok(vec![Self::write(motor_frame(level, level))]);
    }

    let slot = if feature_index == 1 { 1 } else { 0 };
    let before = [
      self.motors[0].load(Ordering::Relaxed),
      self.motors[1].load(Ordering::Relaxed),
    ];
    let mut after = before;
    after[slot] = level;
    self.motors[slot].store(level, Ordering::Relaxed);

    let running = |m: [u8; 2]| (m[0] > 0, m[1] > 0);
    let was_running = before != [0, 0];
    let switching = was_running && after != [0, 0] && running(before) != running(after);
    if !switching {
      return Ok(vec![Self::write(motor_frame(after[0], after[1]))]);
    }

    // Eclipse: zero now, the new motors after the pause, unless a newer command replaced it.
    let hardware = self.hardware.clone();
    let current = self.generation.clone();
    buttplug_core::spawn!("Velvet motor switch", async move {
      async_manager::sleep(MOTOR_SWITCH_SETTLE).await;
      if current.load(Ordering::SeqCst) != generation {
        return;
      }
      if let Err(e) = hardware
        .write_value(&HardwareWriteCmd::new(
          &[VELVET_PROTOCOL_UUID],
          Endpoint::Tx,
          motor_frame(after[0], after[1]),
          false,
        ))
        .await
      {
        warn!("Velvet: delayed motor frame failed (device probably disconnected): {:?}", e);
      }
    });
    Ok(vec![Self::write(motor_frame(0, 0))])
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let level = level.clamp(0, 2) as u8;
    Ok(vec![Self::write(vec![
      0xb0,
      0x04,
      if level == 0 { 0x00 } else { 0x01 },
      0x00,
      level,
      0x00,
    ])])
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    let mut events = device.event_stream();
    async move {
      device
        .subscribe(&HardwareSubscribeCmd::new(feature_id, Endpoint::Rx))
        .await?;
      device
        .write_value(&HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          vec![0xb0, 0x00],
          false,
        ))
        .await?;
      // The toy also pushes its battery unprompted (~every 15 s); either reply will do.
      while let Ok(event) = events.recv().await {
        match event {
          HardwareEvent::Notification(_, endpoint, data) => {
            if endpoint != Endpoint::Rx || data.len() < 6 || data[0] != 0xb0 || data[1] != 0x00 {
              continue;
            }
            let pct = data[5].min(100);
            return Ok(InputReadingV4::new(
              device_index,
              feature_index,
              InputTypeReading::Battery(InputValue::new(pct)),
            ));
          }
          HardwareEvent::Disconnected(_) => break,
        }
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "Velvet".to_owned(),
        "Velvet toy disconnected while reading the battery.".to_owned(),
      ))
    }
    .boxed()
  }
}
