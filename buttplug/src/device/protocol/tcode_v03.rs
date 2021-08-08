use super::{
  generic_command_manager::GenericCommandManager, ButtplugDeviceResultFuture, ButtplugProtocol,
  ButtplugProtocolCommandHandler,
};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{protocol::ButtplugProtocolProperties, DeviceImpl, DeviceWriteCmd, Endpoint},
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct TCodeV03 {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  manager: Arc<Mutex<GenericCommandManager>>,
}

impl ButtplugProtocol for TCodeV03 {
  fn new_protocol(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Box<dyn ButtplugProtocol>
  where
    Self: Sized,
  {
    let manager = Arc::new(Mutex::new(GenericCommandManager::new(&message_attributes)));
    Box::new(Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: vec![],
      manager,
    })
  }
}

impl ButtplugProtocolCommandHandler for TCodeV03 {
  fn handle_linear_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::LinearCmd,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      let mut fut_vec = vec![];
      for v in msg.vectors() {
        let position = (v.position * 99f64) as u32;

        let command = format!("L{}{:02}I{}\n", v.index, position, v.duration);
        fut_vec.push(device.write_value(DeviceWriteCmd::new(
          Endpoint::Tx,
          command.as_bytes().to_vec(),
          false,
        )));
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    let manager = self.manager.clone();
    Box::pin(async move {
      // Store off result before the match, so we drop the lock ASAP.
      let result = manager.lock().await.update_vibration(&msg, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        for (i, cmd) in cmds.iter().enumerate() {
          if let Some(speed) = cmd {
            let tcode_vibrate_cmd = format!("V{}{:02}\n", i, speed).as_bytes().to_vec();
            fut_vec.push(device.write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              tcode_vibrate_cmd,
              false,
            )));
          }
        }
      }
      for fut in fut_vec {
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}
