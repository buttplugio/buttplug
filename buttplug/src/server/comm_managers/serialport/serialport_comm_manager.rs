use crate::{
    core::errors::ButtplugError,
    server::comm_managers::{
        DeviceCommunicationEvent, DeviceCommunicationManager, DeviceCommunicationManagerCreator,
    },
};
use async_std::sync::Sender;
use async_trait::async_trait;

pub struct SerialPortCommunicationManagerCreator {}

impl DeviceCommunicationManagerCreator for SerialPortCommunicationManagerCreator {
    fn new(sender: Sender<DeviceCommunicationEvent>) -> Self {
        Self {}
    }
}

pub struct SerialPortCommunicationManager {}

#[async_trait]
impl DeviceCommunicationManager for SerialPortCommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError> {
        Ok(())
    }

    async fn stop_scanning(&mut self) -> Result<(), ButtplugError> {
        Ok(())
    }

    fn is_scanning(&mut self) -> bool {
        false
    }
}
