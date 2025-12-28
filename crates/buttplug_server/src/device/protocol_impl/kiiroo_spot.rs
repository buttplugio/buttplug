// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{self, InputValue, InputReadingV4, InputTypeReading},
};
use buttplug_server_device_config::Endpoint;
use futures::{FutureExt, future::BoxFuture};
use std::{default::Default, sync::Arc};
use uuid::Uuid;

generic_protocol_setup!(KiirooSpot, "kiiroo-spot");

#[derive(Default)]
pub struct KiirooSpot {}

impl ProtocolHandler for KiirooSpot {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0x00, 0xff, 0x00, 0x00, 0x00, speed as u8],
        false,
      )
      .into(),
    ])
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      let battery_reading = message::InputReadingV4::new(
        device_index,
        feature_index,
        InputTypeReading::Battery(InputValue::new(data[0])),
      );
      debug!("Got battery reading: {}", data[0]);
      Ok(battery_reading)
    }
    .boxed()
  }
}
