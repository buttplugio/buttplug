// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::fleshlight_launch_helper::calculate_speed;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{
      self,
      ButtplugDeviceMessage,
      ButtplugServerDeviceMessage,
      Endpoint,
      SensorReadingV4,
      SensorType,
    },
  },
  server::device::{
    hardware::{
      Hardware,
      HardwareCommand,
      HardwareEvent,
      HardwareReadCmd,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use dashmap::DashSet;
use futures::{
  future::{self, BoxFuture},
  FutureExt,
  StreamExt,
};
use std::{
  default::Default,
  pin::Pin,
  sync::{
    atomic::{AtomicU8, Ordering::SeqCst},
    Arc,
  },
};
use tokio::sync::broadcast;

generic_protocol_setup!(KiirooProWand, "kiiroo-prowand");

#[derive(Default)]
pub struct KiirooProWand {}

impl ProtocolHandler for KiirooProWand {
  fn handle_scalar_vibrate_cmd(
    &self,
    _: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0x00,
        0x00,
        0x64,
        if scalar == 0 { 0x00 } else { 0xff },
        scalar as u8,
        scalar as u8,
      ],
      false,
    )
    .into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorReadCmdV4,
  ) -> BoxFuture<Result<SensorReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    let message = message.clone();
    let msg = HardwareReadCmd::new(Endpoint::RxBLEBattery, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      let battery_level = data[0] as i32;
      let battery_reading = message::SensorReadingV4::new(
        message.device_index(),
        *message.feature_index(),
        *message.sensor_type(),
        vec![battery_level],
      );
      debug!("Got battery reading: {}", battery_level);
      Ok(battery_reading)
    }
    .boxed()
  }
}
