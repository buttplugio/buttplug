use super::device::DeviceImpl;
use crate::core::{
    errors::ButtplugError,
    messages::{ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion},
};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
    async fn initialize(&mut self, device: &Box<dyn DeviceImpl>);
    fn box_clone(&self) -> Box<dyn ButtplugProtocol>;
    // TODO Handle raw messages here.
    async fn parse_message(
        &mut self,
        device: &Box<dyn DeviceImpl>,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError>;
}

impl Clone for Box<dyn ButtplugProtocol> {
    fn clone(&self) -> Box<dyn ButtplugProtocol> {
        self.box_clone()
    }
}
