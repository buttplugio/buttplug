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
    hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
    ServerDeviceIdentifier,
  },
  util::sleep,
};
use async_trait::async_trait;
use futures::{future::BoxFuture, FutureExt};
use regex::Regex;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};

// Constants for dealing with the Lovense subscript/write race condition. The
// timeout needs to be VERY long, otherwise this trips up old lovense serial
// adapters.
//
// Just buy new adapters, people.
const LOVENSE_COMMAND_TIMEOUT_MS: u64 = 500;
const LOVENSE_COMMAND_RETRY: u64 = 5;

pub mod setup {
  use crate::server::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct LovenseIdentifierFactory {}

  impl ProtocolIdentifierFactory for LovenseIdentifierFactory {
    fn identifier(&self) -> &str {
      "lovense"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::LovenseIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct LovenseIdentifier {}

fn lovense_model_resolver(type_response: String) -> String {
  let parts = type_response.split(':').collect::<Vec<&str>>();
  if parts.len() < 2 {
    warn!(
      "Lovense Device returned invalid DeviceType info: {}",
      type_response
    );
    return "lovense".to_string();
  }

  let identifier = parts[0].to_owned();
  let version = parts[1].to_owned().parse::<i32>().unwrap_or(0);

  info!("Identified device type {} version {}", identifier, version);

  // Flexer: version must be 3+ to control actuators separately
  if identifier == "EI" && version >= 3 {
    return "EI-FW3".to_string();
  }

  identifier
}

#[async_trait]
impl ProtocolIdentifier for LovenseIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    let mut count = 0;
    hardware
      .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
      .await?;

    loop {
      let msg = HardwareWriteCmd::new(Endpoint::Tx, b"DeviceType;".to_vec(), false);
      hardware.write_value(&msg).await?;

      select! {
        event = event_receiver.recv().fuse() => {
          if let Ok(HardwareEvent::Notification(_, _, n)) = event {
            let type_response = std::str::from_utf8(&n).map_err(|_| ButtplugDeviceError::ProtocolSpecificError("lovense".to_owned(), "Lovense device init got back non-UTF8 string.".to_owned()))?.to_owned();
            debug!("Lovense Device Type Response: {}", type_response);
            let ident = lovense_model_resolver(type_response);
            return Ok((ServerDeviceIdentifier::new(hardware.address(), "lovense", &ProtocolAttributesType::Identifier(ident.clone())), Box::new(LovenseInitializer::new(ident))));
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Lovense".to_owned(),
                "Lovense Device disconnected while getting DeviceType info.".to_owned(),
              ),
            );
          }
        }
        _ = sleep(Duration::from_millis(LOVENSE_COMMAND_TIMEOUT_MS)).fuse() => {
          count += 1;
          if count > LOVENSE_COMMAND_RETRY {
            warn!("Lovense Device timed out while getting DeviceType info. ({} retries)", LOVENSE_COMMAND_RETRY);
            let re = Regex::new(r"LVS-([A-Z]+)\d+").expect("Static regex shouldn't fail");
            if let Some(caps) = re.captures(hardware.name()) {
              info!("Lovense Device identified by BLE name");
              return Ok((ServerDeviceIdentifier::new(hardware.address(), "lovense", &ProtocolAttributesType::Identifier(caps[1].to_string())), Box::new(LovenseInitializer::new(caps[1].to_string()))));
            };
            return Ok((ServerDeviceIdentifier::new(hardware.address(), "lovense", &ProtocolAttributesType::Default), Box::new(LovenseInitializer::new("".to_string()))));
          }
        }
      }
    }
  }
}
pub struct LovenseInitializer {
  device_type: String,
}

impl LovenseInitializer {
  pub fn new(device_type: String) -> Self {
    Self { device_type }
  }
}

#[async_trait]
impl ProtocolInitializer for LovenseInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    attributes: &ProtocolDeviceAttributes,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut protocol = Lovense::default();
    protocol.device_type = self.device_type.clone();

    if let Some(scalars) = attributes.message_attributes.scalar_cmd() {
      protocol.vibrator_count = scalars
        .clone()
        .iter()
        .filter(|x| [ActuatorType::Vibrate, ActuatorType::Oscillate].contains(x.actuator_type()))
        .count();

      // This might need better tuning if other complex Lovenses are released
      // Currently this only applies to the Flexer/Lapis/Solace
      if (protocol.vibrator_count == 2 && scalars.len() > 2)
        || protocol.vibrator_count > 2
        || protocol.device_type == "H"
      {
        protocol.use_mply = true;
      }
    }

    debug!(
      "Device type {} initialized with {} vibrators {}using Mply",
      protocol.device_type,
      protocol.vibrator_count,
      if protocol.use_mply { "" } else { "not " }
    );
    Ok(Arc::new(protocol))
  }
}

#[derive(Default)]
pub struct Lovense {
  rotation_direction: Arc<AtomicBool>,
  vibrator_count: usize,
  use_mply: bool,
  device_type: String,
}

impl ProtocolHandler for Lovense {
  fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
    // For Lovense, we'll just repeat the device type packet and drop the result.
    super::ProtocolKeepaliveStrategy::RepeatPacketStrategy(HardwareWriteCmd::new(
      Endpoint::Tx,
      b"DeviceType;".to_vec(),
      false,
    ))
  }

  fn handle_scalar_cmd(
    &self,
    cmds: &[Option<(ActuatorType, u32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if self.use_mply {
      let mut speeds = cmds
        .iter()
        .map(|x| {
          if let Some(val) = x {
            val.1.to_string()
          } else {
            "-1".to_string()
          }
        })
        .collect::<Vec<_>>();

      if speeds.len() == 1 && self.device_type == "H" {
        speeds.push("20".to_string()); // Max range
      }

      let lovense_cmd = format!("Mply:{};", speeds.join(":")).as_bytes().to_vec();

      return Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        lovense_cmd,
        false,
      )
      .into()]);
    }

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
      //
      // Note that the windowed comparison causes mixed types as well as mixed
      // speeds to fall back to separate commands. This is because the Gravity's
      // thruster on Vibrate2 is independent of Vibrate
      if self.vibrator_count == vibrate_cmds.len()
        && (self.vibrator_count == 1
          || vibrate_cmds
            .windows(2)
            .all(|w| w[0].0 == w[1].0 && w[0].1 == w[1].1))
      {
        let lovense_cmd = format!("Vibrate:{};", vibrate_cmds[0].1)
          .as_bytes()
          .to_vec();
        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      } else {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some((actuator, speed)) = cmd {
            if ![ActuatorType::Vibrate, ActuatorType::Oscillate].contains(actuator) {
              continue;
            }
            let lovense_cmd = format!("Vibrate{}:{};", i + 1, speed).as_bytes().to_vec();
            hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
          }
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
      let lovense_cmd = format!("Air:Level:{};", constrict_cmds[0].1)
        .as_bytes()
        .to_vec();

      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
    }

    Ok(hardware_cmds)
  }

  fn handle_rotate_cmd(
    &self,
    cmds: &[Option<(u32, bool)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let direction = self.rotation_direction.clone();
    let mut hardware_cmds = vec![];
    if let Some(Some((speed, clockwise))) = cmds.get(0) {
      let lovense_cmd = format!("Rotate:{};", speed).as_bytes().to_vec();
      hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
      let dir = direction.load(Ordering::SeqCst);
      // TODO Should we store speed and direction as an option for rotation caching? This is weird.
      if dir != *clockwise {
        direction.store(*clockwise, Ordering::SeqCst);
        hardware_cmds
          .push(HardwareWriteCmd::new(Endpoint::Tx, b"RotateChange;".to_vec(), false).into());
      }
    }
    Ok(hardware_cmds)
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    let mut device_notification_receiver = device.event_stream();
    async move {
      let write_fut = device.write_value(&HardwareWriteCmd::new(
        Endpoint::Tx,
        b"Battery;".to_vec(),
        false,
      ));
      write_fut.await?;
      while let Ok(event) = device_notification_receiver.recv().await {
        match event {
          HardwareEvent::Notification(_, _, data) => {
            if let Ok(data_str) = std::str::from_utf8(&data) {
              debug!("Lovense event received: {}", data_str);
              let len = data_str.len();
              // Depending on the state of the toy, we may get an initial
              // character of some kind, i.e. if the toy is currently vibrating
              // then battery level comes up as "s89;" versus just "89;". We'll
              // need to chop the semicolon and make sure we only read the
              // numbers in the string.
              //
              // Contains() is casting a wider net than we need here, but it'll
              // do for now.
              let start_pos = usize::from(data_str.contains('s'));
              if let Ok(level) = data_str[start_pos..(len - 1)].parse::<u8>() {
                return Ok(
                  message::SensorReading::new(
                    message.device_index(),
                    0,
                    message::SensorType::Battery,
                    vec![level as i32],
                  )
                  .into(),
                );
              }
            }
          }
          HardwareEvent::Disconnected(_) => {
            return Err(ButtplugDeviceError::ProtocolSpecificError(
              "Lovense".to_owned(),
              "Lovense Device disconnected while getting Battery info.".to_owned(),
            ))
          }
        }
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "Lovense".to_owned(),
        "Lovense Device disconnected while getting Battery info.".to_owned(),
      ))
    }
    .boxed()
  }
}
