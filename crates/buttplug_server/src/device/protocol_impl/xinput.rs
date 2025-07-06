// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::Endpoint;
use byteorder::LittleEndian;

use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{self, InputData, InputReadingV4, InputTypeData},
};
use byteorder::WriteBytesExt;
use futures::future::{BoxFuture, FutureExt};
use std::sync::{
  atomic::{AtomicU16, Ordering},
  Arc,
};

generic_protocol_setup!(XInput, "xinput");

#[derive(Default)]
pub struct XInput {
  speeds: [AtomicU16; 2],
}

impl ProtocolHandler for XInput {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: uuid::Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u16, Ordering::Relaxed);
    // XInput is fast enough that we can ignore the commands handed
    // back by the manager and just form our own packet. This means
    // we'll just use the manager's return for command validity
    // checking.
    let mut cmd = vec![];
    if cmd
      .write_u16::<LittleEndian>(self.speeds[1].load(Ordering::Relaxed))
      .is_err()
      || cmd
        .write_u16::<LittleEndian>(self.speeds[0].load(Ordering::Relaxed))
        .is_err()
    {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "XInput".to_owned(),
        "Cannot convert XInput value for processing".to_owned(),
      ));
    }
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      cmd,
      false,
    )
    .into()])
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: uuid::Uuid,
    _sensor_type: message::InputType,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    async move {
      let reading = device
        .read_value(&HardwareReadCmd::new(feature_id, Endpoint::Rx, 0, 0))
        .await?;
      let battery = match reading.data()[0] {
        0 => 0u8,
        1 => 33,
        2 => 66,
        3 => 100,
        _ => {
          return Err(ButtplugDeviceError::DeviceCommunicationError(
            "something went wrong".to_string(),
          ))
        }
      };
      Ok(message::InputReadingV4::new(
        device_index,
        feature_index,
        InputTypeData::Battery(InputData::new(battery)),
      ))
    }
    .boxed()
  }
}
