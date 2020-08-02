mod test_device;
#[cfg(feature = "server")]
mod test_device_comm_manager;

use crate::device::DeviceImplCommand;
use async_channel::Receiver;
pub use test_device::{
  TestDevice,
  TestDeviceEndpointChannel,
  TestDeviceImplCreator,
  TestDeviceInternal,
};
#[cfg(feature = "server")]
pub use test_device_comm_manager::{
  new_bluetoothle_test_device,
  TestDeviceCommunicationManager,
  TestDeviceCommunicationManagerHelper,
};

#[allow(dead_code)]
pub async fn check_recv_value(receiver: &Receiver<DeviceImplCommand>, command: DeviceImplCommand) {
  assert!(!receiver.is_empty());
  assert_eq!(receiver.recv().await.unwrap(), command);
}
