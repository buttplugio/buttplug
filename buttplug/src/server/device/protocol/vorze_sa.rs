// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugDeviceError,
    message::{self, Endpoint},
  },
  server::device::{
    configuration::ProtocolAttributesType,
    hardware::{Hardware, HardwareCommand, HardwareWriteCmd},
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
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};

generic_protocol_initializer_setup!(VorzeSA, "vorze-sa");

#[derive(Default)]
pub struct VorzeSAInitializer {}

#[async_trait]
impl ProtocolInitializer for VorzeSAInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let hwname = hardware.name().to_ascii_lowercase();
    let device_type = if hwname.contains("cyclone") {
      VorzeDevice::Cyclone
    } else if hwname.contains("ufo tw") {
      VorzeDevice::UfoTw
    } else if hwname.contains("ufo") {
      VorzeDevice::Ufo
    } else if hwname.contains("bach") {
      VorzeDevice::Bach
    } else if hwname.contains("rocket") {
      VorzeDevice::Rocket
    } else if hwname.contains("piston") {
      VorzeDevice::Piston
    } else {
      return Err(ButtplugDeviceError::ProtocolNotImplemented(format!(
        "No protocol implementation for Vorze Device {}",
        hardware.name()
      )));
    };
    Ok(Arc::new(VorzeSA::new(device_type)))
  }
}

pub struct VorzeSA {
  previous_position: Arc<AtomicU8>,
  device_type: VorzeDevice,
}

impl VorzeSA {
  pub fn new(device_type: VorzeDevice) -> Self {
    Self {
      previous_position: Arc::new(AtomicU8::new(0)),
      device_type,
    }
  }
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum VorzeDevice {
  Bach = 6,
  Piston = 3,
  Cyclone = 1,
  Rocket = 7,
  Ufo = 2,
  UfoTw = 5,
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

impl ProtocolHandler for VorzeSA {
  fn handle_scalar_vibrate_cmd(
    &self,
    _index: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![{
      HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![
          self.device_type as u8,
          VorzeActions::Vibrate as u8,
          scalar as u8,
        ],
        true,
      )
      .into()
    }])
  }

  fn handle_rotate_cmd(
    &self,
    cmds: &Vec<Option<(u32, bool)>>,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if let Some((speed, clockwise)) = cmds[0] {
      let data: u8 = (clockwise as u8) << 7 | (speed as u8);
      Ok(vec![HardwareWriteCmd::new(
        Endpoint::Tx,
        vec![self.device_type as u8, VorzeActions::Rotate as u8, data],
        true,
      )
      .into()])
    } else {
      Ok(vec![])
    }
  }

  fn handle_linear_cmd(
    &self,
    msg: message::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let v = msg.vectors()[0].clone();

    let previous_position = self.previous_position.load(Ordering::SeqCst);
    let position = v.position * 200f64;
    let distance = (previous_position as f64 - position).abs();

    let speed = get_piston_speed(distance, v.duration as f64);

    self
      .previous_position
      .store(position as u8, Ordering::SeqCst);

    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![self.device_type as u8, position as u8, speed as u8],
      true,
    )
    .into()])
  }

  fn handle_vorze_a10_cyclone_cmd(
    &self,
    msg: message::VorzeA10CycloneCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.handle_rotate_cmd(&vec![Some((msg.speed(), msg.clockwise()))])
  }
}

/*
#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{
      Endpoint,
      LinearCmd,
      RotateCmd,
      RotationSubcommand,
      StopDeviceCmd,
      VectorSubcommand,
      VibrateCmd,
      VibrateSubcommand,
    },
    server::device::{
      hardware::{HardwareCommand, HardwareWriteCmd},
    hardware::communication::test::{
      check_test_recv_empty,
      check_test_recv_value,
      new_bluetoothle_test_device,
    },
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![0x03, 190, 92], true)),
      );
      assert!(check_test_recv_empty(&command_receiver));

      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 150, 0.95)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(
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
        HardwareCommand::Write(HardwareWriteCmd::new(
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
*/
