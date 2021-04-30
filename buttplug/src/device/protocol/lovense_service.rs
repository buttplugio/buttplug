use super::{ButtplugDeviceResultFuture, ButtplugProtocol, ButtplugProtocolCommandHandler};
use crate::{
  core::errors::ButtplugDeviceError,
  device::{ButtplugDeviceEvent, DeviceSubscribeCmd},
};
use crate::{
  core::{
    errors::ButtplugError,
    messages::{
      self, ButtplugDeviceCommandMessageUnion, ButtplugDeviceMessage, DeviceMessageAttributesMap,
    },
  },
  device::{
    protocol::{generic_command_manager::GenericCommandManager, ButtplugProtocolProperties},
    DeviceImpl, DeviceWriteCmd, Endpoint,
  },
};
use futures::{future::BoxFuture, FutureExt};
use futures_timer::Delay;
use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
  },
  time::Duration,
};
use tokio::sync::Mutex;

#[derive(ButtplugProtocolProperties)]
pub struct LovenseService {
  name: String,
  message_attributes: DeviceMessageAttributesMap,
  manager: Arc<Mutex<GenericCommandManager>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnion>,
  rotation_direction: Arc<AtomicBool>,
}

impl ButtplugProtocol for LovenseService {
  // Due to this lacking the ability to take extra fields, we can't pass in our
  // event receiver from the subscription, which we'll need for things like
  // battery readings. Therefore, we expect initialize() to return the protocol
  // itself instead of calling this, which is simply a convenience method for
  // the default implementation anyways.
  fn new_protocol(name: &str, attrs: DeviceMessageAttributesMap) -> Box<dyn ButtplugProtocol> {
    let manager = GenericCommandManager::new(&attrs);
    Box::new(Self {
      name: name.to_owned(),
      message_attributes: attrs,
      stop_commands: manager.get_stop_commands(),
      manager: Arc::new(Mutex::new(manager)),
      rotation_direction: Arc::new(AtomicBool::new(false)),
    })
  }
}

impl ButtplugProtocolCommandHandler for LovenseService {
  fn handle_vibrate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::VibrateCmd,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_rotate_cmd(
    &self,
    device: Arc<DeviceImpl>,
    msg: messages::RotateCmd,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
      Ok(messages::Ok::default().into())
    })
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<DeviceImpl>,
    message: messages::BatteryLevelCmd,
  ) -> ButtplugDeviceResultFuture {
    Box::pin(async move {
        Ok(messages::BatteryLevelReading::new(message.device_index(), 1.0).into())
      }
    )
  }
}
