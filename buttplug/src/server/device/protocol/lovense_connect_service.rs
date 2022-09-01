// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, ActuatorType, ButtplugDeviceMessage, ButtplugServerMessage, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
    protocol::{
      generic_protocol_initializer_setup,
      ProtocolHandler,
      ProtocolIdentifier,
      ProtocolInitializer,
    },
    ServerDeviceIdentifier,
  },
};
use async_trait::async_trait;
use futures::future::{BoxFuture, FutureExt};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use crate::server::device::configuration::ProtocolDeviceAttributes;

generic_protocol_initializer_setup!(LovenseConnectService, "lovense-connect-service");

#[derive(Default)]
pub struct LovenseConnectServiceInitializer {}

#[async_trait]
impl ProtocolInitializer for LovenseConnectServiceInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Arc::new(LovenseConnectService::new(hardware.address())))
  }
}

#[derive(Default)]
pub struct LovenseConnectService {
  address: String,
  rotation_direction: Arc<AtomicBool>,
}

impl LovenseConnectService {
  pub fn new(address: &str) -> Self {
    Self {
      address: address.to_owned(),
      ..Default::default()
    }
  }
}

impl ProtocolHandler for LovenseConnectService {
  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
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
        cmds[0].expect("Already checked existence").1,
        self.address
      )
      .as_bytes()
      .to_vec();
      msg_vec.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
    } else {
      for (i, cmd) in cmds.iter().enumerate() {
        if let Some((_, speed)) = cmd {
          let lovense_cmd = format!("Vibrate{}?v={}&t={}", i + 1, speed, self.address)
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
    cmds: &Vec<Option<(u32, bool)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let mut hardware_cmds = vec![];
    if let Some(Some((speed, clockwise))) = cmds.get(0) {
      let lovense_cmd = format!("/Rotate?v={}&t={}", speed, self.address)
        .as_bytes()
        .to_vec();
      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      let dir = self.rotation_direction.load(Ordering::SeqCst);
      // TODO Should we store speed and direction as an option for rotation caching? This is weird.
      if dir != *clockwise {
        self.rotation_direction.store(*clockwise, Ordering::SeqCst);
        hardware_cmds
          .push(HardwareWriteCmd::new(Endpoint::Tx, b"RotateChange?".to_vec(), false).into());
      }
    }
    Ok(hardware_cmds)
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    msg: message::SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    async move {
      // This is a dummy read. We just store the battery level in the device
      // implementation and it's the only thing read will return.
      let reading = device
        .read_value(&HardwareReadCmd::new(Endpoint::Rx, 0, 0))
        .await?;
      debug!("Battery level: {}", reading.data()[0]);
      Ok(
        message::SensorReading::new(
          msg.device_index(),
          *msg.sensor_index(),
          *msg.sensor_type(),
          vec![reading.data()[0] as i32],
        )
        .into(),
      )
    }
    .boxed()
  }
}
