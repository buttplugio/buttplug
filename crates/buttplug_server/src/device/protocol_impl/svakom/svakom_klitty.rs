// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
    generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, util::async_manager};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::{
  sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
  },
  time::Duration,
};
use uuid::{Uuid, uuid};

generic_protocol_initializer_setup!(SvakomKlitty, "svakom-klitty");

const SVAKOM_KLITTY_PROTOCOL_UUID: Uuid = uuid!("62e5336b-bb9e-4528-9310-5a524c76b779");
const KEEPALIVE_INTERVAL_MS: u64 = 50;
const STOP_BURST_FRAMES: u8 = (2000 / KEEPALIVE_INTERVAL_MS) as u8;
const HANDSHAKE_GAP_MS: u64 = 80;

const HANDSHAKE: [[u8; 7]; 3] = [
  [0x55, 0x04, 0x00, 0x00, 0x01, 0xFF, 0xAA],
  [0x55, 0x04, 0x00, 0x00, 0x00, 0x00, 0xAA],
  [0x55, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
];

struct MotorState {
  speed: AtomicU8,
  stop_burst_remaining: AtomicU8,
}

impl MotorState {
  fn new() -> Self {
    Self {
      speed: AtomicU8::new(0),
      stop_burst_remaining: AtomicU8::new(0),
    }
  }
}

fn motor_packet(feature_index: u32, speed: u8) -> [u8; 7] {
  match feature_index {
    0 => [
      0x55,
      0x03,
      0x00,
      0x00,
      if speed == 0 { 0x00 } else { 0x01 },
      speed,
      0x00,
    ],
    1 => [0x55, 0x09, 0x00, 0x00, speed, 0x00, 0x00],
    2 => [
      0x55,
      0x14,
      0x00,
      0x00,
      if speed == 0 { 0x00 } else { 0x01 },
      speed,
      0x00,
    ],
    _ => [0x55, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00],
  }
}

fn write_cmd(feature_id: Uuid, packet: [u8; 7]) -> HardwareCommand {
  HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, packet.to_vec(), false).into()
}

async fn klitty_update_loop(hardware: Arc<Hardware>, motors: Arc<[MotorState; 3]>) {
  loop {
    async_manager::sleep(Duration::from_millis(KEEPALIVE_INTERVAL_MS)).await;

    for (feature_index, motor) in motors.iter().enumerate() {
      let speed = motor.speed.load(Ordering::Relaxed);
      let packet = if speed > 0 {
        motor_packet(feature_index as u32, speed)
      } else {
        let remaining = motor.stop_burst_remaining.load(Ordering::Relaxed);
        if remaining == 0 {
          continue;
        }
        motor
          .stop_burst_remaining
          .store(remaining - 1, Ordering::Relaxed);
        motor_packet(feature_index as u32, 0)
      };

      if hardware
        .write_value(&HardwareWriteCmd::new(
          &[SVAKOM_KLITTY_PROTOCOL_UUID],
          Endpoint::Tx,
          packet.to_vec(),
          false,
        ))
        .await
        .is_err()
      {
        return;
      }
    }
  }
}

#[derive(Default)]
pub struct SvakomKlittyInitializer {}

#[async_trait]
impl ProtocolInitializer for SvakomKlittyInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    hardware
      .subscribe(&HardwareSubscribeCmd::new(
        SVAKOM_KLITTY_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await?;

    for (index, packet) in HANDSHAKE.iter().enumerate() {
      if index > 0 {
        async_manager::sleep(Duration::from_millis(HANDSHAKE_GAP_MS)).await;
      }
      hardware
        .write_value(&HardwareWriteCmd::new(
          &[SVAKOM_KLITTY_PROTOCOL_UUID],
          Endpoint::Tx,
          packet.to_vec(),
          false,
        ))
        .await?;
    }

    Ok(Arc::new(SvakomKlitty::new(hardware)))
  }
}

pub struct SvakomKlitty {
  motors: Arc<[MotorState; 3]>,
}

impl SvakomKlitty {
  fn new(hardware: Arc<Hardware>) -> Self {
    let motors = Arc::new([MotorState::new(), MotorState::new(), MotorState::new()]);
    buttplug_core::spawn!(
      "SvakomKlittyUpdateLoop",
      klitty_update_loop(hardware, motors.clone())
    );
    Self { motors }
  }

  fn handle_motor(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let motor = &self.motors[feature_index as usize];
    motor.speed.store(speed as u8, Ordering::Relaxed);
    if speed == 0 {
      motor
        .stop_burst_remaining
        .store(STOP_BURST_FRAMES.saturating_sub(1), Ordering::Relaxed);
    } else {
      motor.stop_burst_remaining.store(0, Ordering::Relaxed);
    }
    Ok(vec![write_cmd(
      feature_id,
      motor_packet(feature_index, speed as u8),
    )])
  }
}

impl ProtocolHandler for SvakomKlitty {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_motor(feature_index, feature_id, speed)
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_motor(feature_index, feature_id, speed)
  }
}
