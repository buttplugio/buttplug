// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::fleshlight_launch_helper::calculate_speed;
use crate::{
  core::{
    errors::ButtplugDeviceError,
    messages::{self, ButtplugDeviceMessage, Endpoint},
  },
  server::device::{
    hardware::{HardwareCommand, HardwareWriteCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
};
use std::sync::{
  atomic::{AtomicBool, Ordering::SeqCst},
  Arc,
};

generic_protocol_setup!(LovenseConnectService, "lovense-connect-service");

#[derive(Default)]
pub struct LovenseConnectService {
  rotation_direction: Arc<AtomicBool>,
}

impl ProtocolHandler for LovenseConnectService {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Lovense is the same situation as the Lovehoney Desire, where commands
    // are different if we're addressing all motors or seperate motors.
    // Difference here being that there's Lovense variants with different
    // numbers of motors.
    //
    // Neat way of checking if everything is the same via
    // https://sts10.github.io/2019/06/06/is-all-equal-function.html.
    //
    // Just make sure we're not matching on None, 'cause if that's the case
    // we ain't got shit to do.
    let mut msg_vec = vec![];
    if cmds[0].is_some() && (cmds.len() == 1 || cmds.windows(2).all(|w| w[0] == w[1])) {
      let lovense_cmd = format!(
        "Vibrate?v={}&t={}",
        cmds[0].expect("Already checked existence"),
        device.address()
      )
      .as_bytes()
      .to_vec();
      msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
    } else {
      for (i, cmd) in cmds.iter().enumerate() {
        if let Some(speed) = cmd {
          let lovense_cmd = format!("Vibrate{}?v={}&t={}", i + 1, speed, device.address())
            .as_bytes()
            .to_vec();
          msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
        }
      }
    }
    Ok(msg_vec)
  }

  fn handle_rotate_cmd(
    &self,
    message: &Vec<Option<(u32, bool)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut msg_vec = vec![];
    let lovense_cmd = format!("/Rotate?v={}&t={}", speed, device.address())
      .as_bytes()
      .to_vec();
    msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
    let dir = self.rotation_direction.load(Ordering::SeqCst);
    // TODO Should we store speed and direction as an option for rotation caching? This is weird.
    if dir != clockwise {
      self.rotation_direction.store(clockwise, Ordering::SeqCst);
      msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, b"RotateChange?".to_vec(), false).into());
    }
    Ok(msg_vec)
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugServerResultFuture {
    Box::pin(async move {
      // This is a dummy read. We just store the battery level in the device
      // implementation and it's the only thing read will return.
      let reading = device
        .read_value(HardwareReadCmd::new(Endpoint::Rx, 0, 0))
        .await?;
      debug!("Battery level: {}", reading.data()[0]);
      Ok(
        messages::BatteryLevelReading::new(
          message.device_index(),
          reading.data()[0] as f64 / 100f64,
        )
        .into(),
      )
    })
  }
}
