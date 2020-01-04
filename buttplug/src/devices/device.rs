use crate::{
    core::{
        errors::ButtplugError,
        messages::{
            self, ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, RawReadCmd,
            RawReading, RawWriteCmd
        },
    },
    devices::{protocol::ButtplugProtocol, Endpoint},
};
use async_trait::async_trait;

pub enum ButtplugProtocolRawMessage {
    RawWriteCmd(RawWriteCmd),
    RawReadCmd(RawReadCmd),
}

pub enum ButtplugDeviceResponseMessage {
    Ok(messages::Ok),
    Error(messages::Error),
    RawReading(RawReading),
}

pub enum ButtplugDeviceEvent {
    DeviceRemoved(),
    MessageEmitted(),
}

#[async_trait]
pub trait DeviceImpl: Sync + Send {
    fn name(&self) -> String;
    fn address(&self) -> String;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<Endpoint>;
    fn disconnect(&self);
    fn box_clone(&self) -> Box<dyn DeviceImpl>;

    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError>;
    async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError>;
}

impl Clone for Box<dyn DeviceImpl> {
    fn clone(&self) -> Box<dyn DeviceImpl> {
        self.box_clone()
    }
}

pub struct ButtplugDevice {
    protocol: Box<dyn ButtplugProtocol>,
    device: Box<dyn DeviceImpl>,
}

impl Clone for ButtplugDevice {
    fn clone(&self) -> Self {
        ButtplugDevice {
            protocol: self.protocol.clone(),
            device: self.device.clone()
        }
    }
}

impl ButtplugDevice {
    pub fn new(protocol: Box<dyn ButtplugProtocol>, device: Box<dyn DeviceImpl>) -> Self {
        Self { protocol, device }
    }

    pub fn name(&self) -> String {
        self.device.name()
    }

    pub async fn parse_message(
        &mut self,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.protocol.parse_message(&self.device, message).await
    }
}


