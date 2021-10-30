mod test_device;
#[cfg(feature = "server")]
mod test_device_comm_manager;

use crate::{
  device::DeviceImplCommand,
  util::stream::{iffy_is_empty_check, recv_now},
};
use std::sync::{Arc, Mutex};
pub use test_device::{
  TestDevice, TestDeviceEndpointChannel, TestDeviceImplCreator, TestDeviceInternal,
};
#[cfg(feature = "server")]
pub use test_device_comm_manager::{
  new_bluetoothle_test_device, TestDeviceCommunicationManager, TestDeviceCommunicationManagerHelper, TestDeviceCommunicationManagerBuilder
};
use tokio::sync::mpsc::Receiver;

#[allow(dead_code)]
pub fn check_test_recv_value(
  receiver: &Arc<Mutex<Receiver<DeviceImplCommand>>>,
  command: DeviceImplCommand,
) {
  assert_eq!(
    recv_now(&mut receiver.lock().expect("Test")).expect("Test").expect("Test"),
    command
  );
}

#[allow(dead_code)]
pub fn check_test_recv_empty(receiver: &Arc<Mutex<Receiver<DeviceImplCommand>>>) -> bool {
  iffy_is_empty_check(&mut receiver.lock().expect("Test"))
}
