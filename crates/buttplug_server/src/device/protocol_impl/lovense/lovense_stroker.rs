// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::InputReadingV4,
  util::{async_manager, sleep},
};
use buttplug_server_device_config::Endpoint;
use futures::future::BoxFuture;
use std::{
  sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
  },
  time::Duration,
};
use uuid::{uuid, Uuid};

const LOVENSE_STROKER_PROTOCOL_UUID: Uuid = uuid!("a97fc354-5561-459a-bc62-110d7c2868ac");

pub struct LovenseStroker {
  linear_info: Arc<(AtomicU32, AtomicU32)>,
}

impl LovenseStroker {
  pub fn new(hardware: Arc<Hardware>) -> Self {
    let linear_info = Arc::new((AtomicU32::new(0), AtomicU32::new(0)));
    async_manager::spawn(update_linear_movement(
      hardware.clone(),
      linear_info.clone(),
    ));
    Self { linear_info }
  }
}

impl ProtocolHandler for LovenseStroker {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    super::keepalive_strategy()
  }

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.linear_info.0.store(position, Ordering::Relaxed);
    self.linear_info.1.store(duration, Ordering::Relaxed);
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
}

async fn update_linear_movement(device: Arc<Hardware>, linear_info: Arc<(AtomicU32, AtomicU32)>) {
  let mut last_goal_position = 0i32;
  let mut current_move_amount = 0i32;
  let mut current_position = 0i32;
  loop {
    // See if we've updated our goal position
    let goal_position = linear_info.0.load(Ordering::Relaxed) as i32;
    // If we have and it's not the same, recalculate based on current status.
    if last_goal_position != goal_position {
      last_goal_position = goal_position;
      // We move every 100ms, so divide the movement into that many chunks.
      // If we're moving so fast it'd be under our 100ms boundary, just move in 1 step.
      let move_steps = (linear_info.1.load(Ordering::Relaxed) / 100).max(1);
      current_move_amount = (goal_position - current_position) / move_steps as i32;
    }

    // If we aren't going anywhere, just pause then restart
    if current_position == last_goal_position {
      sleep(Duration::from_millis(100)).await;
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
    sleep(Duration::from_millis(100)).await;
  }
}
