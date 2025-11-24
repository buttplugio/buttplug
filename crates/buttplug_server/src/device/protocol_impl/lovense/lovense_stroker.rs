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
    Arc, RwLock
  }, time::Duration
};
use uuid::{Uuid, uuid};

use instant::Instant;

const LOVENSE_STROKER_PROTOCOL_UUID: Uuid = uuid!("a97fc354-5561-459a-bc62-110d7c2868ac");

const LINEAR_STEP_INTERVAL: Duration = Duration::from_millis(100);

pub struct LovenseStroker {
  linear_info: Arc<RwLock<(u32, u32, Instant)>>,
}

impl LovenseStroker {
  pub fn new(hardware: Arc<Hardware>) -> Self {
    let linear_info = Arc::new(RwLock::new((0, 0, Instant::now())));
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
    *self.linear_info.write().unwrap() = (position, duration, Instant::now());
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

async fn update_linear_movement(device: Arc<Hardware>, linear_info: Arc<RwLock<(u32, u32, Instant)>>) {
  let mut current_position = 0u32;
  let mut start_position = 0u32;
  let mut last_goal_position = 0u32;
  let mut last_start_time = Instant::now();
  loop {
    let (goal_position, goal_duration, start_time) = { *linear_info.read().unwrap() };
    let current_time = Instant::now();
    let end_time = start_time + Duration::from_millis(goal_duration.try_into().unwrap());

    // Sleep, accounting for time passed during loop (mostly from bt call time)
    let fn_sleep = async || {
      let elapsed = Instant::now() - current_time;
      if elapsed < LINEAR_STEP_INTERVAL {
        sleep(LINEAR_STEP_INTERVAL - elapsed).await
      };
    };

    //debug!("lovense: goal data {:?}/{:?}/{:?}", goal_position, goal_duration, start_time);

    // At rest
    if current_position == goal_position {
      fn_sleep().await;
      continue;
    }

    // If parameters changed, re-capture the current position as the new starting position.
    if last_start_time != start_time || last_goal_position != goal_position {
      start_position = current_position;
      last_start_time = start_time;
      last_goal_position = goal_position;
    }

    // Determine where in the motion we should be
    assert!(current_time >= start_time);
    let step_position = if current_time < end_time {
      let movement_range = goal_position as f64 - start_position as f64;
      let time_elapsed_ms = (current_time - start_time).as_millis();

      let step_percentage = (time_elapsed_ms as f64) / (goal_duration as f64);
      let step_position_dbl = step_percentage * movement_range + (start_position as f64);
      let step_position = step_position_dbl.round() as u32;

      //debug!("lovense: calculating step for time {:?} with start of {:?} and end of {:?}. Pct movement is {:?} from {:?} to {:?}, result {:?}",
      //       current_time, start_time, end_time, step_percentage, start_position, goal_position, step_position);

      step_position
    } else {
      goal_position
    };

    // No movement over this window
    if current_position == step_position {
      fn_sleep().await;
      continue;
    }

    //debug!("lovense: moving to position {:?} from {:?}, goal {:?}", step_position, current_position, goal_position);

    current_position = step_position;

    //let lovense_cmd = format!("FSetSite:{current_position};");
    let lovense_cmd = format!("SetPoint:{current_position};");

    let hardware_cmd: HardwareWriteCmd = HardwareWriteCmd::new(
      &[LOVENSE_STROKER_PROTOCOL_UUID],
      Endpoint::Tx,
      lovense_cmd.into_bytes(),
      false,
    );
    if device.write_value(&hardware_cmd).await.is_err() {
      return;
    }

    fn_sleep().await;
  }
}
