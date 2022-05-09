// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
  util::async_manager,
};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::{Mutex, RwLock};

// Time between Mysteryvibe update commands, in milliseconds. This is basically
// a best guess derived from watching packet timing a few years ago.
//
// Thelemic vibrator. Neat.
//
const MYSTERYVIBE_COMMAND_DELAY_MS: u64 = 93;


pub struct MysteryVibe {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  current_command: Arc<RwLock<Vec<u8>>>,
  updater_running: Arc<AtomicBool>,
}

impl MysteryVibe {
  const PROTOCOL_IDENTIFIER: &'static str = "mysteryvibe";

  fn new(device_attributes: ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      updater_running: Arc::new(AtomicBool::new(false)),
      current_command: Arc::new(RwLock::new(vec![0u8, 0, 0, 0, 0, 0])),
    }
  }
}

#[derive(Default, Debug)]
pub struct MysteryVibeFactory {}

impl ButtplugProtocolFactory for MysteryVibeFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    let msg = DeviceWriteCmd::new(Endpoint::TxMode, vec![0x43u8, 0x02u8, 0x00u8], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      info_fut.await?;
      let device_attributes = builder.create_from_device_impl(&device_impl)?;
      Ok(Box::new(MysteryVibe::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    "mysteryvibe"
  }
}

async fn vibration_update_handler(device: Arc<DeviceImpl>, command_holder: Arc<RwLock<Vec<u8>>>) {
  info!("Entering Mysteryvibe Control Loop");
  let mut current_command = command_holder.read().await.clone();
  while device
    .write_value(DeviceWriteCmd::new(
      Endpoint::TxVibrate,
      current_command,
      false,
    ))
    .await
    .is_ok()
  {
    Delay::new(Duration::from_millis(MYSTERYVIBE_COMMAND_DELAY_MS)).await;
    current_command = command_holder.read().await.clone();
    info!("MV Command: {:?}", current_command);
  }
  info!("Mysteryvibe control loop exiting, most likely due to device disconnection.");
}

crate::default_protocol_properties_definition!(MysteryVibe);

impl ButtplugProtocol for MysteryVibe {}

impl ButtplugProtocolCommandHandler for MysteryVibe {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    let current_command = self.current_command.clone();
    let update_running = self.updater_running.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, true)?;
      info!("MV Result: {:?}", result);
      if result.is_none() {
        return Ok(messages::Ok::default().into());
      }
      let write_mutex = current_command.clone();
      let mut command_writer = write_mutex.write().await;
      let command: Vec<u8> = result
        .expect("Already checked validity")
        .into_iter()
        .map(|x| x.expect("Validity ensured via GCM match_all") as u8)
        .collect();
      *command_writer = command;
      if !update_running.load(Ordering::SeqCst) {
        async_manager::spawn(
          async move { vibration_update_handler(device, current_command).await },
        );
        update_running.store(true, Ordering::SeqCst);
      }
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write some tests!
//
// At least, once I figure out how to do that with the weird timing on this
// thing.
