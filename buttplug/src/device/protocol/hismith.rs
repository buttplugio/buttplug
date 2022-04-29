use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolFactory, ButtplugProtocolCommandHandler};
use crate::{
  core::messages::{self, ButtplugDeviceCommandMessageUnion},
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    configuration_manager::{ProtocolDeviceAttributes, DeviceAttributesBuilder, ProtocolAttributesIdentifier},
    DeviceImpl,
    DeviceReadCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct Hismith {
  device_attributes: ProtocolDeviceAttributes,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
}

impl Hismith {
  const PROTOCOL_IDENTIFIER: &'static str = "hismith";

  fn new(
    device_attributes: ProtocolDeviceAttributes,
  ) -> Self {
    let manager = GenericCommandManager::new(&device_attributes);
    Self {
      device_attributes,
      stop_commands: manager.stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
    }
  }
}

#[derive(Default, Debug)]
pub struct HismithFactory {}

impl ButtplugProtocolFactory for HismithFactory {
  fn try_create(
    &self,
    device_impl: Arc<crate::device::DeviceImpl>,
    builder: DeviceAttributesBuilder,
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
      let device_attributes = builder.create(device_impl.address(), &ProtocolAttributesIdentifier::Identifier(device_identifier), &device_impl.endpoints())?;
      let device = Hismith::new(device_attributes);
      Ok(Box::new(device) as Box<dyn ButtplugProtocol>)
    })
  }

  fn protocol_identifier(&self) -> &'static str {
    Hismith::PROTOCOL_IDENTIFIER
  }
}

impl ButtplugProtocol for Hismith {}

crate::default_protocol_properties_definition!(Hismith);

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
