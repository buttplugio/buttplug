use crate::{
    core::{
        errors::ButtplugError,
        messages::{ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion},
    },
    server::device_manager::DeviceImpl,
};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
    async fn initialize(&mut self);
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
