mod test_device;
mod test_device_comm_manager;

use crate::device::DeviceImplCommand;
use async_channel::Receiver;
pub use test_device::{TestDevice, TestDeviceImplCreator, TestDeviceInternal, TestDeviceEndpointChannel};
pub use test_device_comm_manager::{TestDeviceCommunicationManager, TestDeviceCommunicationManagerHelper, new_bluetoothle_test_device};

#[allow(dead_code)]
pub async fn check_recv_value(receiver: &Receiver<DeviceImplCommand>, command: DeviceImplCommand) {
  assert!(!receiver.is_empty());
  assert_eq!(receiver.recv().await.unwrap(), command);
}
