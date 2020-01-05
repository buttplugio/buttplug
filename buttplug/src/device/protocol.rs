use super::{
    device::DeviceImpl,
};
use crate::core::{
    errors::ButtplugError,
    messages::{ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion},
};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocolCreator: Sync + Send  {
    async fn try_create_protocol(&self, device_impl: &Box<dyn DeviceImpl>) -> Result<Box<dyn ButtplugProtocol>, ButtplugError>;
}

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
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
