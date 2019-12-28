use crate::{
    server::device_manager::{ ButtplugProtocolRawMessage, ButtplugDeviceResponseMessage, DeviceImpl },
    core::{
        messages::{ButtplugMessageUnion, ButtplugDeviceCommandMessageUnion},
        errors::ButtplugError,
    }
};
use async_std::sync::{Sender, Receiver};
use async_trait::async_trait;

pub trait ButtplugProtocolInitializer: Sync + Send {
    fn new(receiver: Receiver<ButtplugDeviceResponseMessage>, sender: Sender<ButtplugProtocolRawMessage>) -> Self;
}

#[async_trait]
pub trait ButtplugProtocol: Sync + Send {
    async fn initialize(&mut self);
    // TODO Handle raw messages here.
    async fn parse_message(&mut self, device: &Box<dyn DeviceImpl>,  message: &ButtplugDeviceCommandMessageUnion) -> Result<ButtplugMessageUnion, ButtplugError>;
}
