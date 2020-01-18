pub mod lovense;
pub mod picobong;

use super::device::DeviceImpl;
use crate::core::{
    errors::ButtplugError,
    messages::{ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, MessageAttributesMap},
};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocolCreator: Sync + Send {
    async fn try_create_protocol(
        &self,
        device_impl: &Box<dyn DeviceImpl>,
    ) -> Result<Box<dyn ButtplugProtocol>, ButtplugError>;
}

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
    fn name(&self) -> &str;
    fn message_attributes(&self) -> MessageAttributesMap;
    fn box_clone(&self) -> Box<dyn ButtplugProtocol>;
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
