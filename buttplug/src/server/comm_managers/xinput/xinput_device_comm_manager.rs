use super::xinput_device_impl::XInputDeviceImplCreator;
use crate::core::errors::ButtplugError;
use crate::server::comm_managers::{
  DeviceCommunicationEvent,
  DeviceCommunicationManager,
  DeviceCommunicationManagerCreator,
};
use async_channel::Sender;
use async_trait::async_trait;

#[derive(Debug, Display, Clone, Copy)]
#[repr(u8)]
pub enum XInputControllerIndex {
  XInputController0 = 0,
  XInputController1 = 1,
  XInputController2 = 2,
  XInputController3 = 3,
}

pub struct XInputDeviceCommunicationManager {
  sender: Sender<DeviceCommunicationEvent>,
  attached_controllers: Vec<XInputControllerIndex>,
}

impl DeviceCommunicationManagerCreator for XInputDeviceCommunicationManager {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      attached_controllers: vec![],
    }
  }
}

#[async_trait]
impl DeviceCommunicationManager for XInputDeviceCommunicationManager {
  async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
    info!("XInput manager scanning!");
    let handle = rusty_xinput::XInputHandle::load_default().unwrap();
    for i in &[
      XInputControllerIndex::XInputController0,
      XInputControllerIndex::XInputController1,
      XInputControllerIndex::XInputController2,
      XInputControllerIndex::XInputController3,
    ] {
      match handle.get_state(*i as u32) {
        Ok(_) => {
          info!("XInput manager found device {}", i);
          let device_creator = Box::new(XInputDeviceImplCreator::new(*i));
          self
            .sender
            .send(DeviceCommunicationEvent::DeviceFound(device_creator))
            .await;
        }
        Err(_) => continue,
      }
    }
    Ok(())
  }

  async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
    Ok(())
  }

  fn is_scanning(&mut self) -> bool {
    false
  }
}
