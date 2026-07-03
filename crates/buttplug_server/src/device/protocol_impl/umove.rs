// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use async_trait::async_trait;
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::util::async_manager;
use buttplug_server_device_config::Endpoint;
use buttplug_server_device_config::{
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use std::{
  sync::{
    Arc,
    atomic::{AtomicU16, AtomicU32, Ordering},
  },
  time::Duration,
};
use uuid::{Uuid, uuid};

const UMOVE_PROTOCOL_UUID: Uuid = uuid!("64afeb97-26ed-4c8e-b67a-5ae43dd2865d");

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct UmoveIdentifierFactory {}

  impl ProtocolIdentifierFactory for UmoveIdentifierFactory {
    fn identifier(&self) -> &str {
      "umove"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::UmoveIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct UmoveIdentifier {}

#[async_trait]
impl ProtocolIdentifier for UmoveIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _specifier: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let device_identifier = hardware.name()[2..4].to_string();
    Ok((
      UserDeviceIdentifier::new(hardware.address(), "umove", &Some(device_identifier)),
      Box::new(UmoveInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct UmoveInitializer {}

#[async_trait]
impl ProtocolInitializer for UmoveInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let state = Arc::new(Umove::default());
    buttplug_core::spawn!(
      "Umove update linear movement",
      update_linear_movement(hardware.clone(), state.clone(),)
    );
    Ok(state)
  }
}

#[derive(Default)]
pub struct Umove {
  packet_id: AtomicU32,
  vibrate: AtomicU16,
  goal_position: AtomicU32,
  current_position: AtomicU32,
  duration: AtomicU32,
}

async fn update_linear_movement(device: Arc<Hardware>, state: Arc<Umove>) {
  let mut last_goal_position = 0i32;
  let mut current_move_amount = 0i32;
  let mut current_position = 0i32;
  loop {
    // See if we've updated our goal position
    let goal_position = state.goal_position.load(Ordering::Relaxed) as i32;
    // If we have and it's not the same, recalculate based on current status.
    if last_goal_position != goal_position {
      last_goal_position = goal_position;
      // We move every 100ms, so divide the movement into that many chunks.
      // If we're moving so fast it'd be under our 100ms boundary, just move in 1 step.
      let move_steps = (state.duration.load(Ordering::Relaxed) / 100).max(1);
      let distance = goal_position - current_position;
      current_move_amount = distance / move_steps as i32;
      if current_move_amount == 0 {
        current_move_amount = distance.signum();
      }
    }

    // If we aren't going anywhere, just pause then restart
    if current_position == last_goal_position {
      async_manager::sleep(Duration::from_millis(100)).await;
      continue;
    }

    // Update our position, make sure we don't overshoot
    current_position += current_move_amount;
    if current_move_amount < 0 {
      if current_position < last_goal_position {
        current_position = last_goal_position;
      }
    } else if current_position > last_goal_position {
      current_position = last_goal_position;
    }
    state
      .current_position
      .store(current_position as u32, Ordering::Relaxed);

    let hardware_cmd: HardwareWriteCmd = HardwareWriteCmd::new(
      &[UMOVE_PROTOCOL_UUID],
      Endpoint::Tx,
      form_command(state.as_ref()),
      false,
    );
    if device.write_value(&hardware_cmd).await.is_err() {
      return;
    }
    async_manager::sleep(Duration::from_millis(50)).await;
  }
}

fn form_command(state: &Umove) -> Vec<u8> {
  let mut data = vec![0x5A, 0xA5, 0x55, 0x00];
  data.append(&mut state.vibrate.load(Ordering::Relaxed).to_le_bytes().to_vec());
  data.append(&mut 1u16.to_le_bytes().to_vec());
  data.append(
    &mut state
      .packet_id
      .fetch_add(1u32, Ordering::Relaxed)
      .to_le_bytes()
      .to_vec(),
  );
  data.append(
    &mut state
      .current_position
      .load(Ordering::Relaxed)
      .to_le_bytes()
      .to_vec(),
  );
  info!("Formed command: {:?}", data);
  data
}

impl ProtocolHandler for Umove {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(Duration::from_millis(500))
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.vibrate.store(speed as u16, Ordering::Relaxed);
    if self.current_position.load(Ordering::Relaxed) != self.goal_position.load(Ordering::Relaxed) {
      return Ok(vec![]);
    }
    Ok(vec![
      HardwareWriteCmd::new(
        &[UMOVE_PROTOCOL_UUID],
        Endpoint::Tx,
        form_command(self),
        false,
      )
      .into(),
    ])
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.goal_position.store(position, Ordering::Relaxed);
    self.duration.store(duration, Ordering::Relaxed);
    Ok(vec![])
  }
  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.goal_position.store(position, Ordering::Relaxed);
    self.current_position.store(position, Ordering::Relaxed);
    self.duration.store(0, Ordering::Relaxed);
    Ok(vec![
      HardwareWriteCmd::new(
        &[UMOVE_PROTOCOL_UUID],
        Endpoint::Tx,
        form_command(self),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0x5a,
          0xa5,
          0x55,
          0x06,
          0xff,
          0xff,
          0x00,
          0x00,
          0xff,
          0xff,
          0xff,
          0xff,
          level as u8,
          0xff,
        ],
        false,
      )
      .into(),
    ])
  }
}
