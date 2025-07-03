// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_server_device_config::{ProtocolCommunicationSpecifier, DeviceDefinition, UserDeviceIdentifier, Endpoint};
use buttplug_core::{
  errors::ButtplugDeviceError,
  util::{async_manager, sleep},
};
use uuid::{uuid, Uuid};
  
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_initializer_setup, ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, ProtocolKeepaliveStrategy},
};
use async_trait::async_trait;
use std::sync::{atomic::{AtomicU8, Ordering}, Arc, RwLock};
use std::time::Duration;

const JOYHUB_PROTOCOL_UUID: Uuid = uuid!("c0f6785a-0056-4a2a-a2a9-dc7ca4ae2a0d");

generic_protocol_initializer_setup!(JoyHub, "joyhub");

async fn delayed_constrict_handler(device: Arc<Hardware>, scalar: u8) {
  sleep(Duration::from_millis(25)).await;
  let res = device
    .write_value(&HardwareWriteCmd::new(
      &[JOYHUB_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0xa0,
        0x07,
        if scalar == 0 { 0x00 } else { 0x01 },
        0x00,
        scalar,
        0xff,
      ],
      false,
    ))
    .await;
  if res.is_err() {
    error!("Delayed JoyHub Constrict command error: {:?}", res.err());
  }
}



#[derive(Default)]
pub struct JoyHubInitializer {}

#[async_trait]
impl ProtocolInitializer for JoyHubInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    //Ok(Arc::new(JoyHub::new(hardware)))
    Ok(Arc::new(JoyHub::default()))
  }
}

#[derive(Default)]
pub struct JoyHub {
  //device: Arc<Hardware>,
  //last_cmds: RwLock<Vec<Option<(ActuatorType, i32)>>>,
  last_cmds: [AtomicU8; 3]
}

impl JoyHub {
  /*
  fn new(device: Arc<Hardware>) -> Self {
    //let last_cmds = RwLock::new(vec![]);
    //Self { device, last_cmds }
  }
  */

  fn form_hardware_command(&self, index: u32, speed: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.last_cmds[index as usize].store(speed as u8, Ordering::Relaxed);
    Ok(vec![HardwareWriteCmd::new(
      &[JOYHUB_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        self.last_cmds[0].load(Ordering::Relaxed),
        self.last_cmds[2].load(Ordering::Relaxed),
        self.last_cmds[1].load(Ordering::Relaxed),
        0x00,
        0xaa,
      ],
      false,
    ).into()])
  }
}

impl ProtocolHandler for JoyHub {

  fn handle_output_vibrate_cmd(
      &self,
      feature_index: u32,
      _feature_id: Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)
  }

  fn handle_output_rotate_cmd(
      &self,
      feature_index: u32,
      _feature_id: Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)
  }

  fn handle_output_oscillate_cmd(
      &self,
      feature_index: u32,
      _feature_id: Uuid,
      speed: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.form_hardware_command(feature_index, speed)    
  }

  fn handle_output_constrict_cmd(
      &self,
      _feature_index: u32,
      feature_id: Uuid,
      level: u32,
    ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![
        0xa0,
        0x07,
        if level == 0 { 0x00 } else { 0x01 },
        0x00,
        level as u8,
        0xff,
      ],
      false,
    )
    .into()])
  }

  /*
  fn handle_value_cmd(
    &self,
    commands: &[Option<(ActuatorType, i32)>],
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let cmd1 = commands[0];
    let mut cmd2 = if commands.len() > 1 {
      commands[1]
    } else {
      None
    };
    let cmd3 = if commands.len() > 2 {
      commands[2]
    } else {
      None
    };

    if let Some(cmd) = cmd2 {
      if cmd.0 == ActuatorType::Constrict {
        cmd2 = None;
        if !scalar_changed(&self.last_cmds, commands, 1usize) {
          // no-op
        } else if vibes_changed(&self.last_cmds, commands, vec![1usize]) {
          let dev = self.device.clone();
          async_manager::spawn(async move { delayed_constrict_handler(dev, cmd.1 as u8).await });
        } else {
          let mut command_writer = self.last_cmds.write().expect("Locks should work");
          *command_writer = commands.to_vec();

          return Ok(vec![HardwareWriteCmd::new(
            Endpoint::Tx,
            vec![
              0xa0,
              0x07,
              if cmd.1 == 0 { 0x00 } else { 0x01 },
              0x00,
              cmd.1 as u8,
              0xff,
            ],
            false,
          )
          .into()]);
        }
      }
    }

    let mut command_writer = self.last_cmds.write().expect("Locks should work");
    *command_writer = commands.to_vec();
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![
        0xa0,
        0x03,
        cmd1.unwrap_or((ActuatorType::Oscillate, 0)).1 as u8,
        cmd3.unwrap_or((ActuatorType::Rotate, 0)).1 as u8,
        cmd2.unwrap_or((ActuatorType::Oscillate, 0)).1 as u8,
        0x00,
        0xaa,
      ],
      false,
    )
    .into()])
  }
  */
}
