// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Buttplug Device Manager, manages Device Subtype (Platform/Communication bus
//! specific) Managers

use crate::core::errors::ButtplugError;
use async_trait::async_trait;
use crate::core::messages::{ self, RawReading, RawWriteCmd, RawReadCmd, ButtplugMessageUnion };
use crate::devices::{
    protocol::ButtplugProtocol,
    Endpoint
};
use async_std::sync::{ Sender, Receiver, channel };

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

pub enum DeviceCommunicationEvent {
    DeviceAdded(ButtplugDevice),
    ScanningFinished()
}

pub struct ButtplugDevice {
    protocol: Box<dyn ButtplugProtocol>,
    device: Box<dyn DeviceImpl>
}

impl ButtplugDevice {
    pub fn new(protocol: Box<dyn ButtplugProtocol>, device: Box<dyn DeviceImpl>) -> Self {
        Self {
            protocol,
            device
        }
    }

    pub async fn parse_message(&mut self, message: &ButtplugMessageUnion) -> ButtplugMessageUnion {
        self.protocol.parse_message(&self.device, message).await;
        ButtplugMessageUnion::Ok(messages::Ok::default())
    }
}

// Storing this in a Vec<Box<dyn T>> causes a associated function issue due to
// the lack of new. Just create an extra trait for defining comm managers.
pub trait DeviceCommunicationManagerCreator: Sync + Send {
    fn new(sender: Sender<DeviceCommunicationEvent>) -> Self;
}

#[async_trait]
pub trait DeviceCommunicationManager: Sync + Send {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError>;
    async fn stop_scanning(&mut self) -> Result<(), ButtplugError>;
    fn is_scanning(&mut self) -> bool;
    // Events happen via channel senders passed to the comm manager.
}

#[async_trait]
pub trait DeviceImpl: Sync + Send {
    fn name(&self) -> String;
    fn address(&self) -> String;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<Endpoint>;
    fn disconnect(&self);

    async fn read_value(&self, msg: &RawReadCmd) -> Result<RawReading, ButtplugError>;
    async fn write_value(&self, msg: &RawWriteCmd) -> Result<(), ButtplugError>;
}

pub struct DeviceManager {
    comm_managers: Vec<Box<dyn DeviceCommunicationManager>>,
    receiver: Receiver<DeviceCommunicationEvent>,
    sender: Sender<DeviceCommunicationEvent>
}

impl DeviceManager {
    pub fn new() -> Self {
        let (sender, receiver) = channel(256);
        Self {
            sender,
            receiver,
            comm_managers: vec!()
        }
    }

    pub fn add_comm_manager<T>(&mut self)
    where T: 'static + DeviceCommunicationManager + DeviceCommunicationManagerCreator {
        self.comm_managers.push(Box::new(T::new(self.sender.clone())));
    }
}

#[cfg(test)]
mod test {
    use super::DeviceManager;
    use crate::server::comm_managers::rumble_ble_comm_manager::RumbleBLECommunicationManager;

    #[test]
    pub fn test_device_manager_creation() {
        let mut dm = DeviceManager::new();
        dm.add_comm_manager::<RumbleBLECommunicationManager>();
    }
}
