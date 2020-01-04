use crate::{
    core::{
        errors::ButtplugError,
        messages::{
            ButtplugDeviceCommandMessageUnion, ButtplugMessageUnion, RawReadCmd,
            RawReading, RawWriteCmd, SubscribeCmd, UnsubscribeCmd
        },
    },
    device::{protocol::ButtplugProtocol, Endpoint},
};
use async_trait::async_trait;
use async_std::sync::Receiver;

pub enum DeviceImplCommand {
    // Endpoint, data, write with response
    Write(Endpoint, Vec<u8>, bool),
    // Endpoint, length, timeout in ms
    Read(Endpoint, u32, u32),
    Subscribe(Endpoint),
    Unsubscribe(Endpoint),
}

impl From<RawReadCmd> for DeviceImplCommand {
    fn from(msg: RawReadCmd) -> Self {
        DeviceImplCommand::Read(msg.endpoint, msg.expected_length, msg.timeout)
    }
}

impl From<RawWriteCmd> for DeviceImplCommand {
    fn from(msg: RawWriteCmd) -> Self {
        DeviceImplCommand::Write(msg.endpoint, msg.data, msg.write_with_response)
    }
}

impl From<SubscribeCmd> for DeviceImplCommand {
    fn from(msg: SubscribeCmd) -> Self {
        DeviceImplCommand::Subscribe(msg.endpoint)
    }
}

impl From<UnsubscribeCmd> for DeviceImplCommand {
    fn from(msg: UnsubscribeCmd) -> Self {
        DeviceImplCommand::Unsubscribe(msg.endpoint)
    }
}

#[derive(Debug)]
pub enum ButtplugDeviceEvent {
    DeviceRemoved(),
    Notification(Endpoint, Vec<u8>),
}

#[async_trait]
pub trait DeviceImpl: Sync + Send {
    fn name(&self) -> String;
    fn address(&self) -> String;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<Endpoint>;
    fn disconnect(&self);
    fn box_clone(&self) -> Box<dyn DeviceImpl>;
    fn get_event_receiver(&self) -> Receiver<ButtplugDeviceEvent>;

    // TODO Taking messages mean we have to form full messages in the protocol.
    // This seems silly. We can probably make stripped down versions to send
    // that don't have message IDs or device indexes.
    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError>;
    async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError>;
    async fn subscribe(&self, msg: &SubscribeCmd) -> Result<(), ButtplugError>;
    async fn unsubscribe(&self, msg: &UnsubscribeCmd) -> Result<(), ButtplugError>;
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

    pub async fn initialize(&mut self) {
        self.protocol.initialize(&self.device).await;
    }

    pub async fn parse_message(
        &mut self,
        message: &ButtplugDeviceCommandMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugError> {
        self.protocol.parse_message(&self.device, message).await
    }
}


