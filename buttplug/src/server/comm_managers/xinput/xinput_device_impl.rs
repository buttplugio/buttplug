use super::xinput_device_comm_manager::XInputControllerIndex;
use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    messages::RawReading,
  },
  device::{
    configuration_manager::{DeviceSpecifier, ProtocolDefinition, XInputSpecifier},
    BoundedDeviceEventBroadcaster,
    ButtplugDeviceImplCreator,
    DeviceImpl,
    DeviceReadCmd,
    DeviceSubscribeCmd,
    DeviceUnsubscribeCmd,
    DeviceWriteCmd,
    Endpoint,
  },
};
use async_trait::async_trait;
use broadcaster::BroadcastChannel;
use byteorder::{LittleEndian, ReadBytesExt};
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
    protocol: ProtocolDefinition,
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

#[async_trait]
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

  async fn disconnect(&mut self) {
  }

  fn box_clone(&self) -> Box<dyn DeviceImpl> {
    Box::new((*self).clone())
  }

  fn get_event_receiver(&self) -> BoundedDeviceEventBroadcaster {
    self.event_receiver.clone()
  }

  async fn read_value(&self, msg: DeviceReadCmd) -> Result<RawReading, ButtplugError> {
    panic!("We should never get here!");
  }

  async fn write_value(&self, msg: DeviceWriteCmd) -> Result<(), ButtplugError> {
    let mut cursor = Cursor::new(msg.data);
    let left_motor_speed = cursor.read_u16::<LittleEndian>().unwrap();
    let right_motor_speed = cursor.read_u16::<LittleEndian>().unwrap();
    self
      .handle
      .set_state(self.index as u32, left_motor_speed, right_motor_speed)
      .map_err(|e: XInputUsageError| {
        ButtplugError::ButtplugDeviceError(ButtplugDeviceError::new(&format!("{:?}", e).to_owned()))
      })
  }

  async fn subscribe(&self, msg: DeviceSubscribeCmd) -> Result<(), ButtplugError> {
    panic!("We should never get here!");
  }

  async fn unsubscribe(&self, msg: DeviceUnsubscribeCmd) -> Result<(), ButtplugError> {
    panic!("We should never get here!");
  }
}
