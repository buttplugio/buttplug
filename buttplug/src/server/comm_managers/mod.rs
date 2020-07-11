#[cfg(feature = "btleplug-manager")]
pub mod btleplug;
#[cfg(all(feature = "xinput", target_os = "windows"))]
pub mod xinput;
#[cfg(all(feature = "xinput", target_os = "windows"))]
use rusty_xinput::XInputUsageError;
#[cfg(feature = "btleplug-manager")]
use ::btleplug::Error as BtleplugError;
use crate::{core::ButtplugResultFuture, device::ButtplugDeviceImplCreator};
use async_channel::Sender;
use thiserror::Error;
use std::{
  hash::Hash,
  sync::{Arc, atomic::AtomicBool}
};

pub enum DeviceCommunicationEvent {
  // This event only means that a device has been found. The work still needs
  // to be done to make sure we can use it.
  DeviceFound(Box<dyn ButtplugDeviceImplCreator>),
  DeviceManagerAdded(Arc<AtomicBool>),
  ScanningFinished,
}

// Storing this in a Vec<Box<dyn T>> causes a associated function issue due to
// the lack of new. Just create an extra trait for defining comm managers.
pub trait DeviceCommunicationManagerCreator: Send {
  fn new(sender: Sender<DeviceCommunicationEvent>) -> Self;
}

pub trait DeviceCommunicationManager: Send + Sync {
  fn name(&self) -> &'static str;
  fn start_scanning(&self) -> ButtplugResultFuture;
  fn stop_scanning(&self) -> ButtplugResultFuture;
  fn scanning_status(&self) -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
  }
  // Events happen via channel senders passed to the comm manager.
}

#[derive(Error, Debug, Clone)]
pub enum ButtplugDeviceSpecificError {
  // XInput library doesn't derive error on its error enum. :(
  #[error("XInput usage error: {0:?}")]
  XInputError(XInputUsageError),
  // Btleplug library uses Failure, not Error, on its error enum. :(
  #[error("Btleplug error: {0:?}")]
  BtleplugError(BtleplugError)
}