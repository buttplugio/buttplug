// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use byteorder::LittleEndian;

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, ActuatorType, ButtplugDeviceMessage, ButtplugServerMessage, Endpoint},
  },
  server::device::{
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};
use byteorder::WriteBytesExt;
use futures::future::{BoxFuture, FutureExt};
use std::sync::Arc;

generic_protocol_setup!(XInput, "xinput");

#[derive(Default)]
pub struct XInput {}

impl ProtocolHandler for XInput {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // XInput is fast enough that we can ignore the commands handed
    // back by the manager and just form our own packet. This means
    // we'll just use the manager's return for command validity
    // checking.
    let mut cmd = vec![];
    if cmd
      .write_u16::<LittleEndian>(
        cmds[1]
          .expect("GCM uses match_all, we'll always get 2 values")
          .1 as u16,
      )
      .is_err()
      || cmd
        .write_u16::<LittleEndian>(
          cmds[0]
            .expect("GCM uses match_all, we'll always get 2 values")
            .1 as u16,
        )
        .is_err()
    {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "XInput".to_owned(),
        "Cannot convert XInput value for processing".to_owned(),
      ));
    }
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, cmd, false).into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    msg: message::SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    async move {
      let reading = device
        .read_value(&HardwareReadCmd::new(Endpoint::Rx, 0, 0))
        .await?;
      let battery = match reading.data()[0] {
        0 => 0i32,
        1 => 33,
        2 => 66,
        3 => 100,
        _ => {
          return Err(ButtplugDeviceError::DeviceCommunicationError(
            "something went wrong".to_string(),
          ))
        }
      };
      Ok(
        message::SensorReading::new(
          msg.device_index(),
          *msg.sensor_index(),
          *msg.sensor_type(),
          vec![battery],
        )
        .into(),
      )
    }
    .boxed()
  }
}
