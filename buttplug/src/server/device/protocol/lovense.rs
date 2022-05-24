// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError},
    messages::{
      self,
      Endpoint,
      ButtplugDeviceMessage,
      ButtplugServerMessage,
    },
  },
  server::{
    device::{
      ServerDeviceIdentifier,
      protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
      configuration::ProtocolAttributesType,
      hardware::{Hardware, HardwareWriteCmd, HardwareEvent, HardwareSubscribeCmd, HardwareCommand},
    },
  },
};
use futures::{future::BoxFuture, FutureExt};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use async_trait::async_trait;

// Constants for dealing with the Lovense subscript/write race condition. The
// timeout needs to be VERY long, otherwise this trips up old lovense serial
// adapters.
//
// Just buy new adapters, people.
const LOVENSE_COMMAND_TIMEOUT_MS: u64 = 500;
const LOVENSE_COMMAND_RETRY: u64 = 5;

pub mod setup {
  use crate::server::device::protocol::{
    ProtocolIdentifier, ProtocolIdentifierFactory,
  };
  #[derive(Default)]
  pub struct LovenseIdentifierFactory {}

  impl ProtocolIdentifierFactory for  LovenseIdentifierFactory {
    fn identifier(&self) -> &str {
      "lovense"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::LovenseIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct LovenseIdentifier {
}

#[async_trait]
impl ProtocolIdentifier for LovenseIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<(ServerDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let mut event_receiver = hardware.event_stream();
    let identifier;
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
            info!("Lovense Device Type Response: {}", type_response);
            identifier = type_response.split(':').collect::<Vec<&str>>()[0].to_owned();
            return Ok((ServerDeviceIdentifier::new(hardware.address(), "lovense", &ProtocolAttributesType::Identifier(identifier)), Box::new(LovenseInitializer::default())));
          } else {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Lovense".to_owned(),
                "Lovense Device disconnected while getting DeviceType info.".to_owned(),
              )
              .into(),
            );
          }
        }
        _ = Delay::new(Duration::from_millis(LOVENSE_COMMAND_TIMEOUT_MS)).fuse() => {
          count += 1;
          if count > LOVENSE_COMMAND_RETRY {
            warn!("Lovense Device timed out while getting DeviceType info. ({} retries)", LOVENSE_COMMAND_RETRY);
            return Ok((ServerDeviceIdentifier::new(hardware.address(), "lovense", &ProtocolAttributesType::Default), Box::new(LovenseInitializer::default())));
          }
        }
      }
    }
  }
}

#[derive(Default)]
pub struct LovenseInitializer {}

#[async_trait]
impl ProtocolInitializer for LovenseInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
  ) -> Result<Box<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(Box::new(Lovense::default()))
  }
}

#[derive(Default)]
pub struct Lovense {
  rotation_direction: Arc<AtomicBool>,
}

impl ProtocolHandler for Lovense {
  fn handle_vibrate_cmd(
    &self,
    cmds: &Vec<Option<u32>>
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
      if cmds[0].is_some() && (cmds.len() == 1 || cmds.windows(2).all(|w| w[0] == w[1])) {
        let lovense_cmd = format!("Vibrate:{};", cmds[0].expect("Already checked validity"))
          .as_bytes()
          .to_vec();
        return Ok(vec!(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into()));
      }
      let mut hardware_cmds = vec!();
      for (i, cmd) in cmds.iter().enumerate() {
        if let Some(speed) = cmd {
          let lovense_cmd = format!("Vibrate{}:{};", i + 1, speed).as_bytes().to_vec();
          hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
        }
      }
      Ok(hardware_cmds)
  }

  fn handle_rotate_cmd(
    &self,
    cmds: &Vec<Option<(u32, bool)>>
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let direction = self.rotation_direction.clone();
    let mut hardware_cmds = vec!();
      if let Some(Some((speed, clockwise))) = cmds.get(0) {
        let lovense_cmd = format!("Rotate:{};", speed).as_bytes().to_vec();
        hardware_cmds.push(HardwareWriteCmd::new(Endpoint::Tx, lovense_cmd, false).into());
        let dir = direction.load(Ordering::SeqCst);
        // TODO Should we store speed and direction as an option for rotation caching? This is weird.
        if dir != *clockwise {
          direction.store(*clockwise, Ordering::SeqCst);
          hardware_cmds.push(HardwareWriteCmd::new(
            Endpoint::Tx,
            b"RotateChange;".to_vec(),
            false,
          ).into());
        }
      }
      Ok(hardware_cmds)
    }

    // TODO Figure out how we're going to reimplement this?!
  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::BatteryLevelCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>>  {
    let mut device_notification_receiver = device.event_stream();
    Box::pin(async move {
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
              let start_pos = if data_str.contains('s') { 1 } else { 0 };
              if let Ok(level) = data_str[start_pos..(len - 1)].parse::<u8>() {
                return Ok(
                  messages::BatteryLevelReading::new(message.device_index(), level as f64 / 100f64)
                    .into(),
                );
              }
            }
          }
          HardwareEvent::Disconnected(_) => {
            return Err(
              ButtplugDeviceError::ProtocolSpecificError(
                "Lovense".to_owned(),
                "Lovense Device disconnected while getting Battery info.".to_owned(),
              )
              .into(),
            )
          }
          HardwareEvent::Connected(_) => {
            unimplemented!("Shouldn't get here as device will always be connected.");
          }
        }
      }
      Err(
        ButtplugDeviceError::ProtocolSpecificError(
          "Lovense".to_owned(),
          "Lovense Device disconnected while getting Battery info.".to_owned(),
        )
        .into(),
      )
    })
  }
}

// TODO Gonna need to add the ability to set subscribe data in tests before
// writing Lovense tests. Oops.
