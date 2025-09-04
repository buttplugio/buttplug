// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2025 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{self, InputData, InputReadingV4, InputType, InputTypeData},
};
use buttplug_server_device_config::Endpoint;
use futures::{future::BoxFuture, FutureExt};
use std::{default::Default, sync::Arc};
use std::sync::atomic::{AtomicU8, Ordering};
use uuid::{uuid, Uuid};
const KIIROO_POWERSHUOT_PROTOCOL_UUID: Uuid = uuid!("06f49eb9-0dca-42a8-92f0-58634cc017d0");

generic_protocol_setup!(KiirooPowerShot, "kiiroo-powershot");

#[derive(Default)]
pub struct KiirooPowerShot {
  last_cmds: [AtomicU8; 2]
}

impl KiirooPowerShot {
  fn form_hardware_command(&self, index: u32, speed: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_cmds[index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[KIIROO_POWERSHUOT_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0x01,
        0x00,
        0x00,
        self.last_cmds[0].load(Ordering::Relaxed),
        self.last_cmds[1].load(Ordering::Relaxed),
        0x00,
      ],
      true,
    ).into()])
  }
}

impl ProtocolHandler for KiirooPowerShot {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      let battery_reading = message::InputReadingV4::new(
        device_index,
        feature_index,
        InputTypeData::Battery(InputData::new(data[0]))
      );
      debug!("Got battery reading: {}", data[0]);
      Ok(battery_reading)
    }
    .boxed()
  }
}
