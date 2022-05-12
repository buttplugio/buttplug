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
    configuration_manager::{ProtocolDeviceAttributes, ProtocolDeviceAttributesBuilder, ProtocolAttributesIdentifier},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};


pub struct Youou {
  device_attributes: ProtocolDeviceAttributes,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  packet_id: AtomicU8,
}

impl Youou {
  const PROTOCOL_IDENTIFIER: &'static str = "youou";

  fn new(device_attributes: crate::device::configuration_manager::ProtocolDeviceAttributes) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);

    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      packet_id: AtomicU8::new(0),
    }
  }
}

#[derive(Default, Debug)]
pub struct YououFactory {}

impl ButtplugProtocolFactory for YououFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: ProtocolDeviceAttributesBuilder,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    // Youou devices have wildcarded names of VX001_*
    // Force the identifier lookup to VX001_
    Box::pin(async move {
      let device_attributes = builder.create(device_impl.address(), &ProtocolAttributesIdentifier::Identifier("VX001_".to_owned()), &device_impl.endpoints())?;
      Ok(Box::new(Youou::new(device_attributes)) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    Youou::PROTOCOL_IDENTIFIER
  }
}

impl ButtplugProtocol for Youou {}

crate::default_protocol_properties_definition!(Youou);

impl ButtplugProtocolCommandHandler for Youou {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager

    // Byte 2 seems to be a monotonically increasing packet id of some kind
    //
    // Speed seems to be 0-247 or so.
    //
    // Anything above that sets a pattern which isn't what we want here.
    let max_value: f64 = 247.0;
    let speed: u8 = (msg.speeds()[0].speed() * max_value) as u8;
    let state: u8 = if speed > 0 { 1 } else { 0 };

    // Scope the packet id set so we can unlock ASAP.
    let mut data = vec![
      0xaa,
      0x55,
      self.packet_id.load(Ordering::SeqCst),
      0x02,
      0x03,
      0x01,
      speed,
      state,
    ];
    self.packet_id.store(
      self.packet_id.load(Ordering::SeqCst).wrapping_add(1),
      Ordering::SeqCst,
    );
    let mut crc: u8 = 0;

    // Simple XOR of everything up to the 9th byte for CRC.
    for b in data.clone() {
      crc ^= b;
    }

    let mut data2 = vec![crc, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    data.append(&mut data2);

    let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

#[cfg(all(test, feature = "server"))]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    server::device::communication_manager::test::{check_test_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_youou_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("VX001_01234")
        .await
        .expect("Test, assuming infallible");
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .expect("Test, assuming infallible");
      let command_receiver = test_device
        .endpoint_receiver(&Endpoint::Tx)
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xaa,
            0x55,
            0x00,
            0x02,
            0x03,
            0x01,
            (247.0f32 / 2.0f32) as u8,
            0x01,
            0x85,
            0xff,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
          ],
          false,
        )),
      );
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .expect("Test, assuming infallible");
      check_test_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xaa, 0x55, 0x01, 0x02, 0x03, 0x01, 0x00, 0x00, 0xfe, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
          ],
          false,
        )),
      );
    });
  }
}
