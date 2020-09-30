use super::{
  ButtplugDeviceResultFuture,
  ButtplugProtocol,
  ButtplugProtocolCommandHandler,
  ButtplugProtocolCreator,
};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::{
  atomic::{AtomicU8, Ordering},
  Arc,
};
use crate::device::configuration_manager::DeviceProtocolConfiguration;
use crate::core::errors::ButtplugError;
use futures::future::BoxFuture;

#[derive(ButtplugProtocol, ButtplugProtocolProperties)]
pub struct Youou {
  name: String,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  packet_id: AtomicU8,
}

impl Youou {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      packet_id: AtomicU8::new(0),
    }
  }
}

impl ButtplugProtocolCreator for Youou {
  fn new_protocol(name: &str, attrs: MessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    Box::new(Self::new(name, attrs))
  }

  fn try_create(
    _device_impl: &dyn DeviceImpl,
    config: DeviceProtocolConfiguration,
  ) -> BoxFuture<'static, Result<Box<dyn ButtplugProtocol>, ButtplugError>>
    where
        Self: Sized,
  {
    // Youou devices have wildcarded names of VX001_*
    // Force the identifier lookup to VX001_
    let (names, attrs) = config.get_attributes("VX001_").unwrap();
    let name = names.get("en-us").unwrap().clone();

    Box::pin(async move { Ok(Self::new_protocol(&name, attrs)) })
  }
}

impl ButtplugProtocolCommandHandler for Youou {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager

    // Byte 2 seems to be a monotonically increasing packet id of some kind
    //
    // Speed seems to be 0-247 or so.
    //
    // Anything above that sets a pattern which isn't what we want here.
    let max_value: f64 = 247.0;
    let speed: u8 = (msg.speeds[0].speed * max_value) as u8;
    let state: u8 = if speed > 0 { 1 } else { 0 };

    let mut data;
    // Scope the packet id set so we can unlock ASAP.
    data = vec![
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

#[cfg(test)]
mod test {
  use crate::{
    core::messages::{StopDeviceCmd, VibrateCmd, VibrateSubcommand},
    device::{DeviceImplCommand, DeviceWriteCmd, Endpoint},
    test::{check_recv_value, new_bluetoothle_test_device},
    util::async_manager,
  };

  #[test]
  pub fn test_youou_protocol() {
    async_manager::block_on(async move {
      let (device, test_device) = new_bluetoothle_test_device("VX001_01234").await.unwrap();
      device
        .parse_message(VibrateCmd::new(0, vec![VibrateSubcommand::new(0, 0.5)]).into())
        .await
        .unwrap();
      let command_receiver = test_device
        .get_endpoint_channel(&Endpoint::Tx)
        .unwrap()
        .receiver;
      check_recv_value(
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
      )
      .await;
      // Test to make sure we handle packet IDs across protocol clones correctly.
      device
        .parse_message(StopDeviceCmd::new(0).into())
        .await
        .unwrap();
      check_recv_value(
        &command_receiver,
        DeviceImplCommand::Write(DeviceWriteCmd::new(
          Endpoint::Tx,
          vec![
            0xaa, 0x55, 0x01, 0x02, 0x03, 0x01, 0x00, 0x00, 0xfe, 0xff, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
          ],
          false,
        )),
      )
      .await;
    });
  }
}
