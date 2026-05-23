// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::{errors::ButtplugDeviceError, message::InputReadingV4, util::async_manager};
use buttplug_server_device_config::Endpoint;
use futures::future::BoxFuture;
use std::{
  sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
  },
  time::Duration,
};
use uuid::{Uuid, uuid};

const LOVENSE_STROKER_PROTOCOL_UUID: Uuid = uuid!("a97fc354-5561-459a-bc62-110d7c2868ac");

struct LovenseStrokerState {
  goal_position: AtomicU32,
  current_position: AtomicU32,
  duration: AtomicU32,
}

impl LovenseStrokerState {
  fn new() -> Self {
    Self {
      goal_position: AtomicU32::new(0),
      current_position: AtomicU32::new(0),
      duration: AtomicU32::new(0),
    }
  }
}

pub struct LovenseStroker {
  state: Arc<LovenseStrokerState>,
  need_range_zerod: bool,
}

impl LovenseStroker {
  pub fn new(hardware: Arc<Hardware>, need_range_zerod: bool) -> Self {
    let state = Arc::new(LovenseStrokerState::new());
    buttplug_core::spawn!(
      "LovenseStroker update linear movement",
      update_linear_movement(hardware.clone(), state.clone(),)
    );
    Self {
      state,
      need_range_zerod,
    }
  }
}

impl ProtocolHandler for LovenseStroker {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    super::keepalive_strategy()
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.state.duration.store(duration, Ordering::Relaxed);
    self.state.goal_position.store(position, Ordering::Relaxed);
    Ok(vec![])
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'static, Result<InputReadingV4, ButtplugDeviceError>> {
    super::handle_battery_level_cmd(device_index, device, feature_index, feature_id)
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if speed == 0 {
      let current_position = self.state.current_position.load(Ordering::Relaxed);
      self.state.duration.store(0, Ordering::Relaxed);
      self
        .state
        .goal_position
        .store(current_position, Ordering::Relaxed);
    }
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        format!(
          "Mply:{}:{};",
          speed,
          if speed == 0 && self.need_range_zerod {
            0
          } else {
            20
          }
        )
        .as_bytes()
        .to_vec(),
        false,
      )
      .into(),
    ])
  }
}

async fn update_linear_movement(device: Arc<Hardware>, state: Arc<LovenseStrokerState>) {
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

    let lovense_cmd = format!("FSetSite:{current_position};");

    let hardware_cmd: HardwareWriteCmd = HardwareWriteCmd::new(
      &[LOVENSE_STROKER_PROTOCOL_UUID],
      Endpoint::Tx,
      lovense_cmd.into_bytes(),
      false,
    );
    if device.write_value(&hardware_cmd).await.is_err() {
      return;
    }
    async_manager::sleep(Duration::from_millis(100)).await;
  }
}
