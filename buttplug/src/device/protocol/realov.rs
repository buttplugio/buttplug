use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;

super::default_protocol_declaration!(Realov);

impl ButtplugProtocolCommandHandler for Realov {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager
    let speed = (msg.speeds()[0].speed() * 50.0) as u8;
    let msg = DeviceWriteCmd::new(Endpoint::Tx, [0xc5, 0x55, speed, 0xaa].to_vec(), false);
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write Tests
