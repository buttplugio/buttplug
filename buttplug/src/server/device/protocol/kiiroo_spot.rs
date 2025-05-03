// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint, SensorReadingV4},
  },
  server::{device::{
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  }, message::checked_sensor_read_cmd::CheckedSensorReadCmdV4},
};
use futures::{future::BoxFuture, FutureExt};
use std::{default::Default, sync::Arc};

generic_protocol_setup!(KiirooSpot, "kiiroo-spot");

#[derive(Default)]
pub struct KiirooSpot {}

impl ProtocolHandler for KiirooSpot {
  fn handle_value_vibrate_cmd(
    &self,
    _: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x00, 0xff, 0x00, 0x00, 0x00, scalar as u8],
      false,
    )
    .into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: CheckedSensorReadCmdV4,
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
        message.feature_index(),
        message.sensor_type(),
        vec![battery_level],
      );
      debug!("Got battery reading: {}", battery_level);
      Ok(battery_reading)
    }
    .boxed()
  }
}
