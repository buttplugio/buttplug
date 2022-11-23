// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::device::configuration::ProtocolDeviceAttributes;
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

generic_protocol_initializer_setup!(LovenseConnectService, "lovense-connect-service");

#[derive(Default)]
pub struct LovenseConnectServiceInitializer {}

#[async_trait]
impl ProtocolInitializer for LovenseConnectServiceInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &ProtocolDeviceAttributes,
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
    
    let mut hardware_cmds = vec![];

    // Handle vibration commands, these will be by far the most common. Fucking machine oscillation
    // uses lovense vibrate commands internally too, so we can include them here.
    let vibrate_cmds: Vec<&(ActuatorType, u32)> = cmds
      .iter()
      .filter(|x| {
        if let Some(val) = x {
          [ActuatorType::Vibrate, ActuatorType::Oscillate].contains(&val.0)
        } else {
          false
        }
      })
      .map(|x| x.as_ref().expect("Already verified is some"))
      .collect();

    if !vibrate_cmds.is_empty() {
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
      if vibrate_cmds.len() == 1 || vibrate_cmds.windows(2).all(|w| w[0] == w[1]) {
        let lovense_cmd = format!(
          "Vibrate?v={}&t={}",
          cmds[0].expect("Already checked existence").1,
          self.address
        )
        .as_bytes()
        .to_vec();
        return Ok(vec![HardwareWriteCmd::new(
          Endpoint::Tx,
          lovense_cmd,
          false,
        )
        .into()]);
      }
      for (i, cmd) in cmds.iter().enumerate() {
        if let Some((_, speed)) = cmd {
          let lovense_cmd = format!("Vibrate{}?v={}&t={}", i + 1, speed, self.address)
          .as_bytes()
          .to_vec();
          hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
        }
      }
    }

    // Handle constriction commands.
    let constrict_cmds: Vec<&(ActuatorType, u32)> = cmds
      .iter()
      .filter(|x| {
        if let Some(val) = x {
          val.0 == ActuatorType::Constrict
        } else {
          false
        }
      })
      .map(|x| x.as_ref().expect("Already verified is some"))
      .collect();
    if !constrict_cmds.is_empty() {
      // Only the max has a constriction system, and there's only one, so just parse the first command.

      // ~ Sutekh
      // This would be the way to implement the AirIn command except after around 5 AirIn commands
      // the toy will stop accepting them until an AirOut command is sent.
      // Note: The clamp(1, 3) is technically out of spec however Lovense Connect apps dont respond to 0,4,5 steps.
      // I don't like this
      let lovense_cmd = format!("AirIn?v={}&t={}", constrict_cmds[0].1.clamp(1, 3), self.address)
      .as_bytes()
      .to_vec();

      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());

      // ~ Sutekh
      // Lovense Connect constriction notes from Sutekh
      // AirIn/AirOut commands need a v parameter

      // ~ Sutekh
      /* ===== Step Level command hack ===== - This is Okay-ish
       * 
       * Even though the spec says the step count is 0-5. My testing has shown that setting the v parameter to 0,4,5 values will do nothing, so I have clamped the "v" parameter (1-3).
       * Step Level Command Hack: When the input step value is 5 do an AirOut command. This works because 0,4,5 step levels seem to do nothing when sending to Lovense Connect app.
       * Problem: This method does not allow the AirIn command to fully finish
       * You could make the ProtocolHandler trait have a mutable reference to self in the handle_scalar_cmd method and add an internal counter so like every 3 Airin commands u send one AirOut I guess?
       * But I ran into an issue while trying that. I forgot where (sorry), but once u convert all the trait implementations to use mutable self reference there was an error somewhere with 
       * an Arc<> type where u would need to implement Mutex around some object.
      if constrict_cmds[0].1 < 5 {
        let lovense_cmd = format!("AirIn?v={}&t={}", constrict_cmds[0].1.clamp(1, 3), self.address)
        .as_bytes()
        .to_vec();

        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      } else if constrict_cmds[0].1 == 5 {
        // This is used to allow quicker updates to constriction - Sutekh
        // The "v" parameter does not matter here clamp to 1-3
        let lovense_constriction_refresh_cmd = format!("AirOut?v={}&t={}", constrict_cmds[0].1.clamp(1, 3), self.address)
        .as_bytes()
        .to_vec();

        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_constriction_refresh_cmd, false).into());
      }
      */

      // ~ Sutekh
      /* ===== Toggle Constrict On/Off Hack ===== - I like this method the best
       * This hack will address the issue with AirIn being disabled from too many AirIn commands in a row.
       * The idea is that you set the constrict to "toggle" on by sending one AirAuto command. It will infinitely inflate the toy.
       * Then when u want it to stop you send an AirOut command.
       * This toggle hack is what I will use in my fork of buttplug. Though this may be unwanted for buttplug.io
      if constrict_cmds[0].1 < 5 {
        let lovense_cmd = format!("AirAuto?v={}&t={}", constrict_cmds[0].1.clamp(1, 3), self.address)
        .as_bytes()
        .to_vec();

        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      } else if constrict_cmds[0].1 == 5 {
        // This is used to allow quicker updates to constriction - Sutekh
        // The "v" parameter does not matter here clamp to 1-3
        let lovense_constriction_refresh_cmd = format!("AirOut?v={}&t={}", constrict_cmds[0].1.clamp(1, 3), self.address)
        .as_bytes()
        .to_vec();

        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_constriction_refresh_cmd, false).into());
      }
      */
    }

    Ok(hardware_cmds)
  
    /* Note from Sutekh:
     * I removed the code below to keep the handle_scalar_cmd methods for lovense toys somewhat consistent.
     * The patch above is almost the same as the "Lovense" ProtocolHandler implementation.
     * I have changed the commands to the Lovense Connect API format.
     * During my testing of the Lovense Connect app's API it seems that even though Constriction has a step range of 0-5. It only responds to values 1-3.
     */
    
    /*
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
    */
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
