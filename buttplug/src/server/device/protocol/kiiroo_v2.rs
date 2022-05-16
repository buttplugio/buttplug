// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{
  fleshlight_launch_helper::calculate_speed,
  ButtplugProtocol,
  ButtplugProtocolFactory,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::messages::{
    self,
    ButtplugDeviceCommandMessageUnion,
    ButtplugDeviceMessage,
    Endpoint,    
    FleshlightLaunchFW12Cmd,
  },
  server::device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder},
    hardware::{Hardware, HardwareWriteCmd, ButtplugDeviceResultFuture},
  },
};
use std::sync::{
  atomic::{AtomicU8, Ordering::SeqCst},
  Arc,
};
use tokio::sync::Mutex;

pub struct KiirooV2 {
  device_attributes: ProtocolDeviceAttributes,
  _manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  previous_position: Arc<AtomicU8>,
}

crate::default_protocol_properties_definition!(KiirooV2);

impl KiirooV2 {
  const PROTOCOL_IDENTIFIER: &'static str = "kiiroo-v2";

  fn new(device_attributes: ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      _manager: Arc::new(Mutex::new(manager)),
      previous_position: Arc::new(AtomicU8::new(0)),
    }
  }
}

#[derive(Default, Debug)]
pub struct KiirooV2Factory {}

impl ButtplugProtocolFactory for KiirooV2Factory {
  fn try_create(
    &self,
    device_impl: Arc<Hardware>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    let msg = HardwareWriteCmd::new(Endpoint::Firmware, vec![0x0u8], true);
    let info_fut = device_impl.write_value(msg);
    Box::pin(async move {
      info_fut.await?;
      let device_attributes = builder.create_from_device_impl(&device_impl)?;
      Ok(Box::new(KiirooV2::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    KiirooV2::PROTOCOL_IDENTIFIER
  }
}

impl ButtplugProtocol for KiirooV2 {}

impl ButtplugProtocolCommandHandler for KiirooV2 {
  fn handle_linear_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position * 99f64)).abs() / 99f64;
    let fl_cmd = FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position * 99f64) as u8,
      (calculate_speed(distance, v.duration) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(device, fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::FleshlightLaunchFW12Cmd,
  ) -> ButtplugDeviceResultFuture {
    let previous_position = self.previous_position.clone();
    let position = message.position();
    let msg = HardwareWriteCmd::new(
      Endpoint::Tx,
      [message.position(), message.speed()].to_vec(),
      false,
    );
    let fut = device.write_value(msg);
    Box::pin(async move {
      previous_position.store(position, SeqCst);
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{Endpoint, FleshlightLaunchFW12Cmd, LinearCmd, VectorSubcommand},
    server::device::{
      communication::test::{check_test_recv_value, new_bluetoothle_test_device},
      hardware::{HardwareCommand, HardwareWriteCmd},
    },
    util::async_manager,
  };

  #[test]
  pub fn test_kiiroov2_fleshlight_fw12cmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(FleshlightLaunchFW12Cmd::new(0, 50, 50).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![50, 50], false)),
      );
    });
  }

  #[test]
  pub fn test_kiiroov2_linearcmd() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("Launch")
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      device
        .parse_message(LinearCmd::new(0, vec![VectorSubcommand::new(0, 500, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        HardwareCommand::Write(HardwareWriteCmd::new(Endpoint::Tx, vec![49, 19], false)),
      );
    });
  }
}
