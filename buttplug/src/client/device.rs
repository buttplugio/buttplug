// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
    internal::{ButtplugClientMessageFuturePair,
               ButtplugClientMessageFuture,
               ButtplugClientDeviceMessage},
    ButtplugClientResult,
    ButtplugClientError,
};
use crate::core::{
    errors::{ButtplugError, ButtplugMessageError},
    messages::{
        ButtplugMessageUnion, DeviceAdded, DeviceMessageInfo, MessageAttributes, VibrateCmd,
        VibrateSubcommand,
    },
};
use async_std::sync::{Sender, Receiver};
use std::collections::HashMap;

pub struct ButtplugClientDevice {
    pub name: String,
    index: u32,
    pub allowed_messages: HashMap<String, MessageAttributes>,
    message_sender: Sender<ButtplugClientMessageFuturePair>,
    // TODO Use this for disconnects
    event_receiver: Receiver<ButtplugClientDeviceMessage>,
}

unsafe impl Send for ButtplugClientDevice {}
unsafe impl Sync for ButtplugClientDevice {}

impl ButtplugClientDevice {
    pub fn new(
        name: &str,
        index: u32,
        allowed_messages: HashMap<String, MessageAttributes>,
        message_sender: Sender<ButtplugClientMessageFuturePair>,
        event_receiver: Receiver<ButtplugClientDeviceMessage>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            index,
            allowed_messages,
            message_sender,
            event_receiver,
        }
    }

    async fn send_message(&mut self, msg: ButtplugMessageUnion) -> ButtplugMessageUnion {
        let fut = ButtplugClientMessageFuture::default();
        self.message_sender
            .send((
                msg.clone(),
                fut.get_state_clone(),
            ))
            .await;
        fut.await
    }

    async fn send_message_expect_ok(&mut self, msg: ButtplugMessageUnion) -> ButtplugClientResult {
        match self.send_message(msg).await {
            ButtplugMessageUnion::Ok(_) => Ok(()),
            ButtplugMessageUnion::Error(_err) => Err(ButtplugClientError::ButtplugError(ButtplugError::from(_err))),
            _ => Err(ButtplugClientError::ButtplugError(ButtplugError::ButtplugMessageError(ButtplugMessageError {
                message: "Got unexpected message type.".to_owned(),
            })))
        }
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

impl From<(&DeviceAdded,
           Sender<ButtplugClientMessageFuturePair>,
           Receiver<ButtplugClientDeviceMessage>)> for ButtplugClientDevice {
    fn from(msg_sender_tuple: (&DeviceAdded,
                               Sender<ButtplugClientMessageFuturePair>,
                               Receiver<ButtplugClientDeviceMessage>))
            -> Self {
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

impl From<(&DeviceMessageInfo,
           Sender<ButtplugClientMessageFuturePair>,
           Receiver<ButtplugClientDeviceMessage>)> for ButtplugClientDevice {
    fn from(msg_sender_tuple: (&DeviceMessageInfo,
                               Sender<ButtplugClientMessageFuturePair>,
                               Receiver<ButtplugClientDeviceMessage>))
            -> Self {
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
