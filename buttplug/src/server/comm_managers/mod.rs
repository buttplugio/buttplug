#[cfg(feature = "btleplug-manager")]
pub mod btleplug;
#[cfg(all(feature = "xinput", target_os = "windows"))]
pub mod xinput;
use async_std::sync::Sender;
use crate::{
  device::device::ButtplugDeviceImplCreator,
  core::errors::ButtplugError,
};

pub enum DeviceCommunicationEvent {
  // This event only means that a device has been found. The work still needs
  // to be done to make sure we can use it.
  DeviceFound(Box<dyn ButtplugDeviceImplCreator>),
  ScanningFinished,
}

// Storing this in a Vec<Box<dyn T>> causes a associated function issue due to
// the lack of new. Just create an extra trait for defining comm managers.
pub trait DeviceCommunicationManagerCreator: Sync + Send {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self;
}

#[async_trait]
pub trait DeviceCommunicationManager: Sync + Send {
  async fn start_scanning(&mut self) -> Result<(), ButtplugError>;
  async fn stop_scanning(&mut self) -> Result<(), ButtplugError>;
  fn is_scanning(&mut self) -> bool;
  // Events happen via channel senders passed to the comm manager.
}
