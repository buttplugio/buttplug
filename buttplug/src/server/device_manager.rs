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
use crate::devices::protocol::ButtplugProtocol;
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

enum DeviceCommunicationEvent {
    DeviceAdded(ButtplugDevice),
    ScanningFinished()
}

struct ButtplugDevice {
    protocol: Box<dyn ButtplugProtocol>,
    device: Box<dyn DeviceImpl>,
}

impl ButtplugDevice {
    pub fn new<J, K>(mut device: Box<J>, mut protocol: Box<K>) -> Self
    where J: 'static + DeviceImpl, K: 'static + ButtplugProtocol {
        let (protocol_sender, device_receiver) = channel(256);
        let (device_sender, protocol_receiver) = channel(256);
        protocol.set_channel(protocol_receiver, protocol_sender);
        device.set_channel(device_receiver, device_sender);
        Self {
            protocol,
            device
        }
    }

    pub async fn parse_message(&mut self, message: &ButtplugMessageUnion) -> ButtplugMessageUnion {
        self.protocol.parse_message(message).await;
        ButtplugMessageUnion::Ok(messages::Ok::default())
    }
}

#[async_trait]
pub trait DeviceCommunicationManager {
    async fn start_scanning(&mut self) -> Result<(), ButtplugError>;
    async fn stop_scanning(&mut self) -> Result<(), ButtplugError>;
    fn is_scanning(&mut self) -> bool;
    // Events happen via channel senders passed to the comm manager.
}

#[async_trait]
pub trait DeviceImpl {
    fn name(&self) -> String;
    fn address(&self) -> String;
    fn connected(&self) -> bool;
    fn endpoints(&self) -> Vec<String>;
    fn disconnect(&self);
    fn set_channel(&mut self, receiver: Receiver<ButtplugProtocolRawMessage>, sender: Sender<ButtplugDeviceResponseMessage>);
}

// struct DeviceManager {}
