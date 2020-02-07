pub mod aneros;
pub mod kiiroo_gen2;
pub mod kiiroo_gen21;
pub mod kiiroo_gen2vibe;
pub mod lelo_f1s;
pub mod libo_elle;
pub mod libo_kegel;
pub mod libo_shark;
pub mod libo_vibes;
pub mod lovehoney_desire;
pub mod lovense;
pub mod magicmotion1;
pub mod magicmotion2;
pub mod magicmotion3;
pub mod maxpro;
pub mod picobong;
pub mod prettylove;
pub mod realov;
pub mod svakom;
pub mod youcups;
pub mod youou;

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
