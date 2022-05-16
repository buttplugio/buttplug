// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    Endpoint,
  },
  server::device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::device_impl::{Hardware, HardwareReadCmd, HardwareWriteCmd},
  },
};
use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};
use tokio::sync::Mutex;

pub struct LovenseConnectService {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  rotation_direction: Arc<AtomicBool>,
}

impl LovenseConnectService {
  const PROTOCOL_IDENTIFIER: &'static str = "lovense-connect-service";

  // Due to this lacking the ability to take extra fields, we can't pass in our
  // event receiver from the subscription, which we'll need for things like
  // battery readings. Therefore, we expect initialize() to return the protocol
  // itself instead of calling this, which is simply a convenience method for
  // the default implementation anyways.
  fn new(device_attributes: ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);
    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      rotation_direction: Arc::new(AtomicBool::new(false)),
    }
  }
}

super::default_protocol_trait_declaration!(LovenseConnectService);
crate::default_protocol_properties_definition!(LovenseConnectService);

impl ButtplugProtocol for LovenseConnectService {}

impl ButtplugProtocolCommandHandler for LovenseConnectService {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, false)?;
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
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        if cmds[0].is_some() && (cmds.len() == 1 || cmds.windows(2).all(|w| w[0] == w[1])) {
          let lovense_cmd = format!(
            "Vibrate?v={}&t={}",
            cmds[0].expect("Already checked existence"),
            device.address()
          )
          .as_bytes()
          .to_vec();
          let fut = device.write_value(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false));
          fut.await?;
          return Ok(messages::Ok::default().into());
        }
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some(speed) = cmd {
            let lovense_cmd = format!("Vibrate{}?v={}&t={}", i + 1, speed, device.address())
              .as_bytes()
              .to_vec();
            fut_vec.push(device.write_value(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false)));
          }
        }
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_rotate_cmd(
    &self,
    device: Arc<Hardware>,
    msg: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    let direction = self.rotation_direction.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_rotation(&msg)?;
      if let Some((speed, clockwise)) = result[0] {
        let lovense_cmd = format!("/Rotate?v={}&t={}", speed, device.address())
          .as_bytes()
          .to_vec();
        let fut = device.write_value(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false));
        fut.await?;
        let dir = direction.load(Ordering::SeqCst);
        // TODO Should we store speed and direction as an option for rotation caching? This is weird.
        if dir != clockwise {
          direction.store(clockwise, Ordering::SeqCst);
          let fut = device.write_value(HardwareWriteCmd::new(
            Endpoint::Tx,
            b"RotateChange?".to_vec(),
            false,
          ));
          fut.await?;
        }
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugDeviceResultFuture {
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
