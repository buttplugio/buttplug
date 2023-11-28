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
    attributes: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut protocol = LovenseConnectService::new(hardware.address());

    if let Some(scalars) = attributes.message_attributes.scalar_cmd() {
      protocol.vibrator_count = scalars
        .clone()
        .iter()
        .filter(|x| [ActuatorType::Vibrate].contains(x.actuator_type()))
        .count();
      protocol.thusting_count = scalars
        .clone()
        .iter()
        .filter(|x| [ActuatorType::Oscillate].contains(x.actuator_type()))
        .count();

      // The Ridge and Gravity both oscillate, but the Ridge only oscillates but takes
      // the vibrate command... The Gravity has a vibe as well, and uses a Thrusting
      // command for that oscillator.
      if protocol.vibrator_count == 0 && protocol.thusting_count != 0 {
        protocol.vibrator_count = protocol.thusting_count;
        protocol.thusting_count = 0;
      }
    }

    if hardware.name() == "Solace" {
      // Just hardcoding this weird exception until we can control depth
      let lovense_cmd = format!("Depth?v={}&t={}", 3, hardware.address())
        .as_bytes()
        .to_vec();

      hardware
        .write_value(&HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into())
        .await?;

      protocol.vibrator_count = 0;
      protocol.thusting_count = 1;
    }

    Ok(Arc::new(protocol))
  }
}

#[derive(Default)]
pub struct LovenseConnectService {
  address: String,
  rotation_direction: Arc<AtomicBool>,
  vibrator_count: usize,
  thusting_count: usize,
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
          if self.thusting_count == 0 {
            [ActuatorType::Vibrate, ActuatorType::Oscillate].contains(&val.0)
          } else {
            [ActuatorType::Vibrate].contains(&val.0)
          }
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
      if self.vibrator_count == vibrate_cmds.len()
        && (self.vibrator_count == 1 || vibrate_cmds.windows(2).all(|w| w[0].1 == w[1].1))
      {
        let lovense_cmd = format!("Vibrate?v={}&t={}", vibrate_cmds[0].1, self.address)
          .as_bytes()
          .to_vec();
        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      } else {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some((actuator, speed)) = cmd {
            if self.thusting_count == 0
              && ![ActuatorType::Vibrate, ActuatorType::Oscillate].contains(actuator)
            {
              continue;
            }
            if self.thusting_count != 0 && ![ActuatorType::Vibrate].contains(actuator) {
              continue;
            }
            let lovense_cmd = format!("Vibrate{}?v={}&t={}", i + 1, speed, self.address)
              .as_bytes()
              .to_vec();
            hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
          }
        }
      }
    }

    // Handle constriction commands.
    let thrusting_cmds: Vec<&(ActuatorType, u32)> = cmds
      .iter()
      .filter(|x| {
        if let Some(val) = x {
          [ActuatorType::Oscillate].contains(&val.0)
        } else {
          false
        }
      })
      .map(|x| x.as_ref().expect("Already verified is some"))
      .collect();
    if self.thusting_count != 0 && !thrusting_cmds.is_empty() {
      let lovense_cmd = format!("Thrusting?v={}&t={}", thrusting_cmds[0].1, self.address)
        .as_bytes()
        .to_vec();

      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
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
      /* ~ Sutekh
       * - Implemented constriction.
       * - Kept things consistent with the lovense handle_scalar_cmd() method.
       * - Using AirAuto method.
       * - Changed step count in device config file to 3.
       */
      let lovense_cmd = format!("AirAuto?v={}&t={}", constrict_cmds[0].1, self.address)
        .as_bytes()
        .to_vec();

      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
    }

    // Handle "rotation" commands: Currently just applicable as the Flexer's Fingering command
    let rotation_cmds: Vec<&(ActuatorType, u32)> = cmds
      .iter()
      .filter(|x| {
        if let Some(val) = x {
          val.0 == ActuatorType::Rotate
        } else {
          false
        }
      })
      .map(|x| x.as_ref().expect("Already verified is some"))
      .collect();

    if !rotation_cmds.is_empty() {
      let lovense_cmd = format!("Fingering?v={}&t={}", rotation_cmds[0].1, self.address)
        .as_bytes()
        .to_vec();

      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
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
    @@ -77,26 +220,27 @@
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
    cmds: &[Option<(u32, bool)>],
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
