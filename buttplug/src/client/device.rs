// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
    internal::{
        ButtplugClientDeviceEvent, ButtplugClientMessageFuture, ButtplugClientMessageFuturePair,
    },
    connectors::ButtplugClientConnectorError,
    ButtplugClientError, ButtplugClientResult,
};
use crate::core::{
    errors::{ButtplugError, ButtplugMessageError, ButtplugDeviceError},
    messages::{
        ButtplugMessageUnion, DeviceAdded, DeviceMessageInfo, MessageAttributes, VibrateCmd,
        VibrateSubcommand,
    },
};
use async_std::{
    prelude::StreamExt,
    sync::{Receiver, Sender}
};
use std::collections::HashMap;

pub struct ButtplugClientDevice {
    pub name: String,
    index: u32,
    pub allowed_messages: HashMap<String, MessageAttributes>,
    message_sender: Sender<ButtplugClientMessageFuturePair>,
    event_receiver: Receiver<ButtplugClientDeviceEvent>,
    events: Vec<ButtplugClientDeviceEvent>,
    device_connected: bool,
    client_connected: bool,
}

unsafe impl Send for ButtplugClientDevice {}
unsafe impl Sync for ButtplugClientDevice {}

impl ButtplugClientDevice {
    pub fn new(
        name: &str,
        index: u32,
        allowed_messages: HashMap<String, MessageAttributes>,
        message_sender: Sender<ButtplugClientMessageFuturePair>,
        event_receiver: Receiver<ButtplugClientDeviceEvent>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            index,
            allowed_messages,
            message_sender,
            event_receiver,
            device_connected: true,
            client_connected: true,
            events: vec!(),
        }
    }

    async fn send_message(&mut self, msg: ButtplugMessageUnion) -> Result<ButtplugMessageUnion, ButtplugClientError> {
        // Since we're using async_std channels, if we send a message and the
        // event loop has shut down, we may never know (and therefore possibly
        // block infinitely) if we don't check the status of an event loop
        // receiver to see if it's returned None. Always run connection/event
        // checks before sending messages to the event loop.
        self.check_for_events().await?;
        let fut = ButtplugClientMessageFuture::default();
        self.message_sender
            .send((msg.clone(), fut.get_state_clone()))
            .await;
        Ok(fut.await)
    }

    async fn send_message_expect_ok(&mut self, msg: ButtplugMessageUnion) -> ButtplugClientResult {
        match self.send_message(msg).await? {
            ButtplugMessageUnion::Ok(_) => Ok(()),
            ButtplugMessageUnion::Error(_err) => Err(ButtplugClientError::ButtplugError(
                ButtplugError::from(_err),
            )),
            _ => Err(ButtplugClientError::ButtplugError(
                ButtplugError::ButtplugMessageError(ButtplugMessageError {
                    message: "Got unexpected message type.".to_owned(),
                }),
            )),
        }
    }

    async fn check_for_events(&mut self) -> ButtplugClientResult {
        if !self.client_connected {
            return Err(ButtplugClientError::from(
                ButtplugClientConnectorError::new("Client not connected.")));
        }
        if !self.device_connected {
            return Err(ButtplugClientError::from(
                ButtplugDeviceError::new("Device not connected.")));
        }
        while !self.event_receiver.is_empty() {
            match self.event_receiver.next().await {
                Some(msg) => {
                    // If this is a disconnect, relay as such.
                    if let ButtplugClientDeviceEvent::DeviceDisconnect = msg {
                        self.device_connected = false;
                    }
                    self.events.push(msg)
                }
                None => {
                    self.client_connected = false;
                    self.device_connected = false;
                    // If we got None, this means the internal loop stopped and our
                    // sender was dropped. We should consider this a disconnect.
                    self.events.push(ButtplugClientDeviceEvent::ClientDisconnect);
                    return Err(ButtplugClientError::from(
                        ButtplugClientConnectorError::new("Client not connected.")));
                }
            }
        }
        Ok(())
    }

    /// Produces a future that will wait for a set of events from the
    /// internal loop. Returns once any number of events is received.
    ///
    /// This should be called whenever the client isn't doing anything
    /// otherwise, so we can respond to unexpected updates from the server, such
    /// as devices connections/disconnections, log messages, etc... This is
    /// basically what event handlers in C# and JS would deal with, but we're in
    /// Rust so this requires us to be slightly more explicit.
    pub async fn wait_for_event(&mut self) -> Result<ButtplugClientDeviceEvent, ButtplugClientError> {
        debug!("Device waiting for event.");
        if !self.client_connected {
            return Err(ButtplugClientError::from(
                ButtplugClientConnectorError::new("Client not connected.")));
        }
        if !self.device_connected {
            return Err(ButtplugClientError::from(
                ButtplugDeviceError::new("Device not connected.")));
        }
        Ok({
            if !self.events.is_empty() {
                self.events.pop().unwrap()
            } else {
                match self.event_receiver.next().await {
                    Some(msg) => msg,
                    None => {
                        // If we got None, this means the internal loop stopped and our
                        // sender was dropped. We should consider this a disconnect.
                        self.client_connected = false;
                        self.device_connected = false;
                        ButtplugClientDeviceEvent::ClientDisconnect
                    }
                }
            }
        })
    }

    pub async fn send_vibrate_cmd(&mut self, speed: f64) -> ButtplugClientResult {
        self.send_message_expect_ok(ButtplugMessageUnion::VibrateCmd(VibrateCmd::new(
            self.index,
            vec![VibrateSubcommand::new(0, speed)],
        )))
        .await
    }

    // pub async fn send_linear_cmd(&self) -> ButtplugClientResult {
    //     None
    // }

    // pub async fn send_rotation_cmd(&self) -> ButtplugClientResult {
    //     None
    // }
}

impl
    From<(
        &DeviceAdded,
        Sender<ButtplugClientMessageFuturePair>,
        Receiver<ButtplugClientDeviceEvent>,
    )> for ButtplugClientDevice
{
    fn from(
        msg_sender_tuple: (
            &DeviceAdded,
            Sender<ButtplugClientMessageFuturePair>,
            Receiver<ButtplugClientDeviceEvent>,
        ),
    ) -> Self {
        let msg = msg_sender_tuple.0.clone();
        ButtplugClientDevice::new(
            &*msg.device_name,
            msg.device_index,
            msg.device_messages,
            msg_sender_tuple.1,
            msg_sender_tuple.2,
        )
    }
}

impl
    From<(
        &DeviceMessageInfo,
        Sender<ButtplugClientMessageFuturePair>,
        Receiver<ButtplugClientDeviceEvent>,
    )> for ButtplugClientDevice
{
    fn from(
        msg_sender_tuple: (
            &DeviceMessageInfo,
            Sender<ButtplugClientMessageFuturePair>,
            Receiver<ButtplugClientDeviceEvent>,
        ),
    ) -> Self {
        let msg = msg_sender_tuple.0.clone();
        ButtplugClientDevice::new(
            &*msg.device_name,
            msg.device_index,
            msg.device_messages,
            msg_sender_tuple.1,
            msg_sender_tuple.2,
        )
    }
}
