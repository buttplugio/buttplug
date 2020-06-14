use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct Maxpro {
  name: String,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Maxpro {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
    }
  }
}

impl ButtplugProtocolCommandHandler for Maxpro {
  fn handle_vibrate_cmd(
    &self,
    device: &dyn DeviceImpl,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    // TODO Convert to using generic command manager

    // Speed range for Maxpro toys are 10-100 for some reason.
    let max_value: f64 = 100.0;
    let speed: u8 = (msg.speeds[0].speed * max_value) as u8;
    let mut data = vec![0x55, 0x04, 0x07, 0xff, 0xff, 0x3f, speed, 0x5f, speed, 0x00];
    let mut crc: u8 = 0;

    for b in data.clone() {
      crc = crc.wrapping_add(b);
    }

    data[9] = crc;

    let msg = DeviceWriteCmd::new(Endpoint::Tx, data, false);
    // device.write_value(msg.into()).await?;
    let fut = device.write_value(msg.into());
    Box::pin(async move {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write some tests! Especially with the weird operational range on this.
