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
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::atomic::{AtomicU8, Ordering::SeqCst};
use std::sync::Arc;
use tokio::sync::Mutex;


pub struct VorzeSA {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

impl VorzeSA {
  const PROTOCOL_IDENTIFIER: &'static str = "vorze-sa";

  fn new(device_attributes: crate::device::configuration_manager::ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    }
  }
}

super::default_protocol_trait_declaration!(VorzeSA);

#[repr(u8)]
#[derive(PartialEq)]
enum VorzeDevices {
  Bach = 6,
  Piston = 3,
  Cyclone = 1,
  Rocket = 7,
  Ufo = 2,
}

#[repr(u8)]
enum VorzeActions {
  Rotate = 1,
  Vibrate = 3,
}

pub fn get_piston_speed(mut distance: f64, mut duration: f64) -> u8 {
  if distance <= 0f64 {
    return 100;
  }

  if distance > 200f64 {
    distance = 200f64;
  }

  // Convert duration to max length
  duration = 200f64 * duration / distance;

  let mut speed = (duration / 6658f64).powf(-1.21);

  if speed > 100f64 {
    speed = 100f64;
  }

  if speed < 0f64 {
    speed = 0f64;
  }

  speed as u8
}

crate::default_protocol_properties_definition!(VorzeSA);

impl ButtplugProtocol for VorzeSA {}

impl ButtplugProtocolCommandHandler for VorzeSA {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    let dev_id = if self.name().to_ascii_lowercase().contains("rocket") {
      VorzeDevices::Rocket
    } else {
      VorzeDevices::Bach
    };
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&msg, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          fut_vec.push(device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![dev_id as u8, VorzeActions::Vibrate as u8, speed as u8],
            true,
          )));
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
    device: Arc<DeviceImpl>,
    msg: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    // This will never change, so we can process it before the future.
    let dev_id = if self.name().contains("UFO") {
      VorzeDevices::Ufo
    } else {
      VorzeDevices::Cyclone
    };
    Box::pin(async move {
      let result = manager.lock().await.update_rotation(&msg)?;
      let mut fut_vec = vec![];
      if let Some((speed, clockwise)) = result[0] {
        let data: u8 = (clockwise as u8) << 7 | (speed as u8);
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![dev_id as u8, VorzeActions::Rotate as u8, data],
          true,
        )));
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_linear_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = msg.vectors()[0].clone();

    let previous_position = self.previous_position.load(SeqCst);
    let position = v.position * 200f64;
    let distance = (previous_position as f64 - position).abs();

    let speed = get_piston_speed(distance, v.duration as f64);

    self.previous_position.store(position as u8, SeqCst);

    let fut = device.write_value(DeviceWriteCmd::new(
      Endpoint::Tx,
      vec![VorzeDevices::Piston as u8, position as u8, speed as u8],
      true,
    ));

    Box::pin(async move {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VorzeA10CycloneCmd,
  ) -> ButtplugDeviceResultFuture {
    self.handle_rotate_cmd(
      device,
      messages::RotateCmd::new(
        msg.device_index(),
        vec![messages::RotationSubcommand::new(
          0,
          msg.speed() as f64 / 99f64,
          msg.clockwise(),
        )],
      ),
    )
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{
      LinearCmd,
      RotateCmd,
      RotationSubcommand,
      StopDeviceCmd,
      VectorSubcommand,
      VibrateCmd,
      VibrateSubcommand,
    },
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::comm_managers::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
    util::async_manager,
  };

  #[test]
  pub fn test_vorze_sa_vibration_protocol_bach() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Bach smart")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x06, 0x03, 50],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x06, 0x03, 0x0],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }

  #[test]
  pub fn test_vorze_sa_vibration_protocol_rocket() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("ROCKET")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x07, 0x03, 50],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x07, 0x03, 0x0],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }

  #[test]
  pub fn test_vorze_sa_rotation_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("CycSA")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, false)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 50],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(RotateCmd::new(0, vec![RotationSubcommand::new(0, 0.5, true)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 178],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x01, 0x01, 0x0],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));
    });
  }

  #[test]
  pub fn test_vorze_sa_linear_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("VorzePiston")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 150, 0.95)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(Endpoint::Tx, vec![0x03, 190, 92], true)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 150, 0.95)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x03, 190, 100],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 50, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![0x03, 100, 100],
          true,
        )),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      assert!(check_test_recv_empty(&command_receiver));
    });
  }
}
