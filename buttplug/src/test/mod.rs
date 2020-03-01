mod test_device;
mod test_device_comm_manager;

pub use test_device::{TestDevice, TestDeviceImplCreator};
pub use test_device_comm_manager::TestDeviceCommunicationManager;
use async_std::sync::Receiver;
use crate::device::device::DeviceImplCommand;

pub async fn check_recv_value(receiver: &Receiver<DeviceImplCommand>, command: DeviceImplCommand) {
    assert!(!receiver.is_empty());
    assert_eq!(receiver.recv().await.unwrap(), command);
}