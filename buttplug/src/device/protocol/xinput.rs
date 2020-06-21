use super::{ButtplugProtocol, ButtplugProtocolCommandHandler, ButtplugProtocolCreator};
use crate::{
  core::{
    errors::{ButtplugError, ButtplugMessageError},
    messages::{self, ButtplugDeviceCommandMessageUnion, MessageAttributesMap},
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
  server::ButtplugServerResultFuture,
};
use async_mutex::Mutex;
use byteorder::{LittleEndian, WriteBytesExt};
use std::sync::Arc;

#[derive(ButtplugProtocol, ButtplugProtocolCreator, ButtplugProtocolProperties)]
pub struct XInput {
  name: String,
  message_attributes: MessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl XInput {
  pub(super) fn new(name: &str, message_attributes: MessageAttributesMap) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);

    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

impl ButtplugProtocolCommandHandler for XInput {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<Box<dyn DeviceImpl>>,
    msg: messages::VibrateCmd,
  ) -> ButtplugServerResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, true);
      // My life for an async closure so I could just do this via and_then(). :(
      match result {
        Ok(cmds_option) => {
          let mut fut_vec = vec![];
          if let Some(cmds) = cmds_option {
            // XInput is fast enough that we can ignore the commands handed
            // back by the manager and just form our own packet. This means
            // we'll just use the manager's return for command validity
            // checking.
            let mut cmd = vec![];
            if cmd
              .write_u16::<LittleEndian>(cmds[1].unwrap() as u16)
              .is_err()
              || cmd
                .write_u16::<LittleEndian>(cmds[0].unwrap() as u16)
                .is_err()
            {
              return Err(ButtplugError::ButtplugMessageError(ButtplugMessageError::new(
                "Cannot convert XInput value for processing",
              )));
            }
            fut_vec.push(device.write_value(DeviceWriteCmd::new(Endpoint::Tx, cmd, false)));
          }

          for fut in fut_vec {
            fut.await?;
          }
          Ok(messages::Ok::default().into())
        }
        Err(e) => Err(e),
      }
    })
  }
}
