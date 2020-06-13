use super::xinput_device_impl::XInputDeviceImplCreator;
use crate::core::ButtplugResultFuture;
use crate::server::comm_managers::{
  DeviceCommunicationEvent,
  DeviceCommunicationManager,
  DeviceCommunicationManagerCreator,
};
use async_channel::Sender;
use futures::future;

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
  _attached_controllers: Vec<XInputControllerIndex>,
}

impl DeviceCommunicationManagerCreator for XInputDeviceCommunicationManager {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
    Self {
      sender,
      _attached_controllers: vec![],
    }
  }
}

impl DeviceCommunicationManager for XInputDeviceCommunicationManager {
  fn start_scanning(&self) -> ButtplugResultFuture {
    info!("XInput manager scanning!");
    let sender = self.sender.clone();
    Box::pin(async move {
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
          if sender
            .send(DeviceCommunicationEvent::DeviceFound(device_creator))
            .await
            .is_err() {

            }
        }
        Err(_) => continue,
      }
    }
    Ok(())
  })
  }

  fn stop_scanning(&self) -> ButtplugResultFuture {
    Box::pin(future::ready(Ok(())))
  }

  fn is_scanning(&self) -> bool {
    false
  }
}
