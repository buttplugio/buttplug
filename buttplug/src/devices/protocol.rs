use crate::{
    server::device_manager::{ ButtplugProtocolRawMessage, ButtplugDeviceResponseMessage },
    core::messages::ButtplugMessageUnion,
};
use async_std::sync::{Sender, Receiver};
use async_trait::async_trait;

#[async_trait]
pub trait ButtplugProtocol {
    // TODO Handle raw messages here.
    async fn parse_message(&mut self, message: &ButtplugMessageUnion);
    fn set_channel(&mut self, receiver: Receiver<ButtplugDeviceResponseMessage>, sender: Sender<ButtplugProtocolRawMessage>);
}

pub struct LovenseProtocol {
    raw_sender: Sender<ButtplugProtocolRawMessage>,
    raw_receiver: Receiver<ButtplugDeviceResponseMessage>,
}
