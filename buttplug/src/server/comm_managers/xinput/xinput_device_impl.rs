use super::xinput_device_comm_manager::XInputControllerIndex;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
    ButtplugResultFuture,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, XInputSpecifier},
    BoundedDeviceEventBroadcaster, ButtplugDeviceImplCreator, DeviceImpl, DeviceReadCmd,
    DeviceSubscribeCmd, DeviceUnsubscribeCmd, DeviceWriteCmd, Endpoint,
  },
};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use byteorder::{LittleEndian, ReadBytesExt};
use futures::future::{self, BoxFuture};
use rusty_xinput::{XInputHandle, XInputUsageError};
use std::io::Cursor;

pub struct XInputDeviceImplCreator {
  index: XInputControllerIndex,
}

impl XInputDeviceImplCreator {
  pub fn new(index: XInputControllerIndex) -> Self {
    debug!("Emitting a new xbox device impl creator!");
    Self { index }
  }
}

#[async_trait]
impl ButtplugDeviceImplCreator for XInputDeviceImplCreator {
  fn get_specifier(&self) -> DeviceSpecifier {
    debug!("Getting the specifier!");
    DeviceSpecifier::XInput(XInputSpecifier::default())
  }

  async fn try_create_device_impl(
    &mut self,
    _protocol: ProtocolDefinition,
  ) -> Result<Box<dyn DeviceImpl>, ButtplugError> {
    debug!("Emitting a new xbox device impl!");
    Ok(Box::new(XInputDeviceImpl::new(self.index)))
  }
}

#[derive(Clone, Debug)]
pub struct XInputDeviceImpl {
  handle: XInputHandle,
  index: XInputControllerIndex,
  event_receiver: BoundedDeviceEventBroadcaster,
  address: String,
}

impl XInputDeviceImpl {
  pub fn new(index: XInputControllerIndex) -> Self {
    let event_receiver = BroadcastChannel::with_cap(256);
    Self {
      handle: rusty_xinput::XInputHandle::load_default().unwrap(),
      index,
      event_receiver,
      address: format!("XInput Controller {}", index).to_owned(),
    }
  }
}

impl DeviceImpl for XInputDeviceImpl {
  fn name(&self) -> &str {
    // This has to match the xinput identifier entry in the configuration
    // file, otherwise things will explode.
    "XInput Gamepad"
  }

  fn address(&self) -> &str {
    &self.address
  }

  fn connected(&self) -> bool {
    true
  }

  fn endpoints(&self) -> Vec<Endpoint> {
    vec![Endpoint::Tx]
  }

  fn disconnect(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_receiver.clone()
  }

  fn read_value(&self, _msg: DeviceReadCmd) -> BoxFuture<'static, Result<RawReading, ButtplugError>> {
    panic!("We should never get here!");
  }

  fn write_value(&self, msg: DeviceWriteCmd) -> ButtplugResultFuture {
    let handle = self.handle.clone();
    let index = self.index;
    Box::pin(async move {
      let mut cursor = Cursor::new(msg.data);
      let left_motor_speed = cursor.read_u16::<LittleEndian>().unwrap();
      let right_motor_speed = cursor.read_u16::<LittleEndian>().unwrap();
      handle
        .set_state(index as u32, left_motor_speed, right_motor_speed)
        .map_err(|e: XInputUsageError| {
          ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(
            &format!("{:?}", e).to_owned(),
          ))
        })
    })
  }

  fn subscribe(&self, _msg: DeviceSubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }

  fn unsubscribe(&self, _msg: DeviceUnsubscribeCmd) -> ButtplugResultFuture {
    panic!("We should never get here!");
  }
}
