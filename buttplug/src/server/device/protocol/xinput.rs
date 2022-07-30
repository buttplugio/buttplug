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
    messages::{self, ActuatorType, ButtplugDeviceMessage, ButtplugServerMessage, Endpoint},
  },
  server::device::{
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};
use byteorder::WriteBytesExt;
use futures::future::BoxFuture;
use std::sync::Arc;

generic_protocol_setup!(XInput, "xinput");

#[derive(Default)]
pub struct XInput {}

impl ProtocolHandler for XInput {
  fn handle_scalar_cmd(
    &self,
    cmds: &Vec<Option<(ActuatorType, u32)>>,
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
      return Err(
        ButtplugDeviceError::ProtocolSpecificError(
          "XInput".to_owned(),
          "Cannot convert XInput value for processing".to_owned(),
        )
        .into(),
      );
    }
    Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, cmd, false).into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    _message: messages::BatteryLevelCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    Box::pin(async move {
      let rawreading = device
        .read_value(&HardwareReadCmd::new(Endpoint::Rx, 0, 0))
        .await?;
      let id = rawreading.device_index();
      let battery = match rawreading.data()[0] {
        0 => 0.0,
        1 => 0.33,
        2 => 0.66,
        3 => 1.0,
        _ => {
          return Err(
            ButtplugDeviceError::DeviceCommunicationError(format!("something went wrong")).into(),
          )
        }
      };
      Ok(ButtplugServerMessage::BatteryLevelReading(
        messages::BatteryLevelReading::new(id, battery),
      ))
    })
  }
}
