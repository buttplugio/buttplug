mod test_device;

pub use test_device::TestDevice;
use async_std::sync::Receiver;
use crate::device::device::DeviceImplCommand;

pub async fn check_recv_value(receiver: &Receiver<DeviceImplCommand>, command: DeviceImplCommand) {
    assert!(!receiver.is_empty());
    assert_eq!(receiver.recv().await.unwrap(), command);
}