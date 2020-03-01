use crate::{
    core::errors::{ButtplugDeviceError, ButtplugError},
    device::device::{ButtplugDevice, ButtplugDeviceImplCreator},
    server::device_manager::{
        DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
    },
};
use super::TestDeviceImplCreator;
use async_trait::async_trait;
use async_std::{
    sync::{Sender, Arc, Mutex},
    task
};
use lazy_static::lazy_static;

lazy_static! {
    // We create device comm manager instances within the buttplug server,
    // meaning we can't actually store devices within an instance, because we
    // may not be able to get it back out. The device list is kept as a module
    // static, so we can add devices without worrying about when/where the comm
    // manager exists.
    static ref DEVICE_LIST: Arc<Mutex<Vec<Box<dyn ButtplugDeviceImplCreator>>>> = Arc::new(Mutex::new(vec!()));
}

pub struct TestDeviceCommunicationManager {
    device_sender: Sender<DeviceCommunicationEvent>,
}

impl TestDeviceCommunicationManager {
    pub fn add_test_device(device_impl_creator: TestDeviceImplCreator) {
        task::block_on(async {
            DEVICE_LIST.lock().await.push(Box::new(device_impl_creator));
        });
    }
}

impl DeviceCommunicationManagerCreator for TestDeviceCommunicationManager {
    fn new(device_sender: Sender<DeviceCommunicationEvent>) -> Self {
        Self {
            device_sender
        }
    }
}

#[async_trait]
impl DeviceCommunicationManager for TestDeviceCommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        let mut dq = task::block_on(async {
            DEVICE_LIST.lock().await
        });
        if dq.is_empty() {
            panic!("No devices for test device comm manager to emit!");
        }
        while let Some(d) = dq.pop() {
            self.device_sender.send(DeviceCommunicationEvent::DeviceFound(d)).await;
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

#[cfg(test)]
mod test {
    #[cfg(test)]
    mod test {
        use crate::{
            core::messages::{self, ButtplugMessageUnion},
            server::{ButtplugServer},
            test::{TestDeviceCommunicationManager, TestDevice},
            device::device::DeviceImpl,
        };
        use async_std::{
            prelude::StreamExt,
            sync::channel,
            task,
        };

        #[test]
        fn test_test_device_comm_manager() {
            let _ = env_logger::builder().is_test(true).try_init();
            let (send, mut recv) = channel(256);
            let mut server = ButtplugServer::new("Test Server", 0, send);
            let (device, device_creator) = TestDevice::new_bluetoothle_test_device_impl_creator("Massage Demo");
            TestDeviceCommunicationManager::add_test_device(device_creator);
            server.add_comm_manager::<TestDeviceCommunicationManager>();
            task::block_on(async {
                let msg = messages::RequestServerInfo::new("Test Client", 1);
                let mut reply = server.parse_message(&msg.into()).await;
                assert!(reply.is_ok(),
                format!("Should get back ok: {:?}", reply));
                reply = server.parse_message(&messages::StartScanning::default().into()).await;
                assert!(reply.is_ok(),
                format!("Should get back ok: {:?}", reply));
                // Check that we got an event back about a new device.
                let msg = recv.next().await.unwrap();
                if let ButtplugMessageUnion::DeviceAdded(da) = msg {
                    assert_eq!(da.device_name, "Aneros Vivi");
                } else {
                    assert!(false, format!("Returned message was not a DeviceAdded message or timed out: {:?}", msg));
                }
                device.disconnect().await;
                // Check that we got an event back about a removed device.
                let msg = recv.next().await.unwrap();
                if let ButtplugMessageUnion::DeviceRemoved(da) = msg {
                    assert_eq!(da.device_index, 0);
                } else {
                    assert!(false, format!("Returned message was not a DeviceRemoved message or timed out: {:?}", msg));
                }
            });
        }
    }
}
