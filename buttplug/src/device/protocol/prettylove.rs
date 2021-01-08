use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::core::errors::ButtplugError;
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceWriteCmd,
    Endpoint,
  },
};
use futures::future::{self, BoxFuture};
use std::sync::Arc;

#[derive(ButtplugProtocolProperties)]
pub struct PrettyLove {
  name: String,
  message_attributes: MessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl ButtplugProtocol for PrettyLove {
  fn new_protocol(
    name: &str,
    message_attributes: MessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&message_attributes);

    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
    })
  }

  fn initialize(
    _device_impl: &dyn DeviceImpl,
  ) -> BoxFuture<'static, Result<Option<String>, ButtplugError>> {
    // Pretty Love devices have wildcarded names of Aogu BLE *
    // Force the identifier lookup to "Aogu BLE"
    Box::pin(future::ready(Ok(Some("Aogu BLE".to_owned()))))
  }
}

impl ButtplugProtocolCommandHandler for PrettyLove {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // TODO Convert to using generic command manager
    let mut speed = (msg.speeds[0].speed * 3.0) as u8;
    if speed == 0 {
      speed = 0xff;
    }
    let msg = DeviceWriteCmd::new(Endpoint::Tx, [0x00, speed].to_vec(), false);
    let fut = device.write_value(msg);
    Box::pin(async {
      fut.await?;
      Ok(messages::Ok::default().into())
    })
  }
}

// TODO Write tests
