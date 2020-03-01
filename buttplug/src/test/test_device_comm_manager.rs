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

