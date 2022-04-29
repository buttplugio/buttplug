use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion, DeviceMessageAttributesMap},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl,
    DeviceReadCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::{Arc};
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct Hismith {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Hismith {
  fn new(
    name: &str,
    message_attributes: DeviceMessageAttributesMap,
  ) -> Self {
    let manager = GenericCommandManager::new(&message_attributes);
    Self {
      name: name.to_owned(),
      message_attributes,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

impl ButtplugProtocol for Hismith {
  fn try_create(
    device_impl: Arc<crate::device::DeviceImpl>,
    config: crate::device::protocol::DeviceProtocolConfiguration,
  ) -> futures::future::BoxFuture<
    'static,
    Result<Box<dyn ButtplugProtocol>, crate::core::errors::ButtplugError>,
  > {
    Box::pin(async move {
      let result = device_impl
          .read_value(DeviceReadCmd::new(Endpoint::RxBLEModel, 128, 500))
          .await?;
      let device_identifier = result.data().into_iter().map( |b| format!("{:02x}", b) ).collect::<String>();
      info!(
        "Hismith Device Identifier: {}",
        device_identifier
      );
      let (name, attrs) = crate::device::protocol::get_protocol_features(
        device_impl.clone(),
        Some(device_identifier),
        config,
      )?;
      let device = Self::new(&name, attrs);
      Ok(Box::new(device) as Box<dyn ButtplugProtocol>)
    })
  }
}

impl ButtplugProtocolCommandHandler for Hismith {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    // Store off result before the match, so we drop the lock ASAP.
    let manager = self.manager.clone();
    Box::pin(async move {
      let result = manager.lock().await.update_vibration(&message, false)?;
      let mut fut_vec = vec![];
      if let Some(cmds) = result {
        if let Some(speed) = cmds[0] {
          fut_vec.push(device.write_value(DeviceWriteCmd::new(
            Endpoint::Tx,
            vec![0xAA, 0x04, speed as u8, (speed + 4) as u8],
            false,
          )));
        }
        if cmds.len() > 1 {
          if let Some(speed) = cmds[1] {
            let value = if speed == 0 { 0xf0 } else { speed };
            fut_vec.push(device.write_value(DeviceWriteCmd::new(
              Endpoint::Tx,
              vec![0xAA, 0x06, value as u8, (value + 6) as u8],
              false,
            )));
          }
        }
      }
      // TODO Just use join_all here
      for fut in fut_vec {
        // TODO Do something about possible errors here
        fut.await?;
      }
      Ok(messages::Ok::default().into())
    })
  }
}