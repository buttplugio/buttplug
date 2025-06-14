// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint, SensorReadingV4, SensorType},
  },
  server::device::{
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};
use futures::{future::BoxFuture, FutureExt};
use std::{default::Default, sync::Arc};
use uuid::Uuid;

generic_protocol_setup!(KiirooSpot, "kiiroo-spot");

#[derive(Default)]
pub struct KiirooSpot {}

impl ProtocolHandler for KiirooSpot {
  fn handle_actuator_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      feature_id,
      Endpoint::Tx,
      vec![0x00, 0xff, 0x00, 0x00, 0x00, speed as u8],
      false,
    )
    .into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<SensorReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      let battery_level = data[0] as i32;
      let battery_reading =
        message::SensorReadingV4::new(0, feature_index, SensorType::Battery, vec![battery_level]);
      debug!("Got battery reading: {}", battery_level);
      Ok(battery_reading)
    }
    .boxed()
  }
}
