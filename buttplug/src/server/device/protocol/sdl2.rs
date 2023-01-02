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

generic_protocol_setup!(SDL2, "sdl2");

#[derive(Default)]
pub struct SDL2 {}

impl ProtocolHandler for SDL2 {
  fn needs_full_command_set(&self) -> bool {
    true
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some(Some((actuator_type, _))) = cmds.iter().find(|cmd| match cmd {
      None => false,
      Some((ActuatorType::Vibrate, _)) => false,
      _ => true,
    }) {
      return Err(ButtplugDeviceError::ProtocolSpecificError(
        "SDL2".to_owned(),
        format!("{actuator_type} actuators are not supported"),
      ));
    };

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
        "SDL2".to_owned(),
        "Cannot convert SDL2 value for processing".to_owned(),
      ));
    }

    Ok(vec![
      HardwareWriteCmd::new(Endpoint::TxVibrate, cmd, false).into()
    ])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    msg: message::SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    async move {
      let reading = device
        .read_value(&HardwareReadCmd::new(Endpoint::RxBLEBattery, 0, 0))
        .await?;

      let data_len = reading.data().len();
      if data_len != 1 {
        return Err(ButtplugDeviceError::ProtocolSpecificError(
          "SDL2".to_owned(),
          format!("Expected 1 byte of battery data, got {data_len}"),
        ));
      }

      let battery = reading.data()[0] as i32;
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
