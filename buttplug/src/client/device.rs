// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
    connectors::ButtplugClientConnectorError,
    internal::{
        ButtplugClientDeviceEvent, ButtplugClientMessageFuture, ButtplugClientMessageFuturePair,
    },
    ButtplugClientError, ButtplugClientResult,
};
use crate::core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    messages::{
        ButtplugMessageUnion, DeviceAdded, DeviceMessageInfo, LinearCmd, MessageAttributes,
        RotateCmd, RotationSubcommand, StopDeviceCmd, VectorSubcommand, VibrateCmd,
        VibrateSubcommand,
    },
};
use async_std::{
    prelude::StreamExt,
    sync::{Receiver, Sender},
};
use std::collections::HashMap;

pub enum VibrateCommand {
    Speed(f64),
    SpeedVec(Vec<f64>),
    SpeedMap(HashMap<u32, f64>),
}

pub enum RotateCommand {
    Rotate(f64, bool),
    RotateVec(Vec<(f64, bool)>),
    RotateMap(HashMap<u32, (f64, bool)>),
}

pub enum LinearCommand {
    Linear(u32, f64),
    LinearVec(Vec<(u32, f64)>),
    LinearMap(HashMap<u32, (u32, f64)>),
}

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
    pub(crate) fn new(
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
            events: vec![],
        }
    }

    async fn send_message(
        &mut self,
        msg: ButtplugMessageUnion,
    ) -> Result<ButtplugMessageUnion, ButtplugClientError> {
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
                ButtplugClientConnectorError::new("Client not connected."),
            ));
        }
        if !self.device_connected {
            return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                "Device not connected.",
            )));
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
                    self.events
                        .push(ButtplugClientDeviceEvent::ClientDisconnect);
                    return Err(ButtplugClientError::from(
                        ButtplugClientConnectorError::new("Client not connected."),
                    ));
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
    pub async fn wait_for_event(
        &mut self,
    ) -> Result<ButtplugClientDeviceEvent, ButtplugClientError> {
        debug!("Device waiting for event.");
        if !self.client_connected {
            return Err(ButtplugClientError::from(
                ButtplugClientConnectorError::new("Client not connected."),
            ));
        }
        if !self.device_connected {
            return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                "Device not connected.",
            )));
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

    pub async fn vibrate(&mut self, speed_cmd: VibrateCommand) -> ButtplugClientResult {
        if !self.allowed_messages.contains_key("VibrateCmd") {
            return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                "Device does not support vibration.",
            )));
        }
        let mut vibrator_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get("VibrateCmd") {
            if let Some(v) = features.feature_count {
                vibrator_count = v;
            }
        }
        let mut speed_vec: Vec<VibrateSubcommand>;
        match speed_cmd {
            VibrateCommand::Speed(speed) => {
                speed_vec = Vec::with_capacity(vibrator_count as usize);
                for i in 0..vibrator_count {
                    speed_vec.push(VibrateSubcommand::new(i, speed));
                }
            }
            VibrateCommand::SpeedMap(map) => {
                if map.len() as u32 > vibrator_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} vibrators, but {} commands were sent.",
                            vibrator_count,
                            map.len()
                        ),
                    )));
                }
                speed_vec = Vec::with_capacity(map.len() as usize);
                for (idx, speed) in map {
                    if idx > vibrator_count - 1 {
                        return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                            &format!(
                                "Max vibrator index is {}, command referenced {}.",
                                vibrator_count, idx
                            ),
                        )));
                    }
                    speed_vec.push(VibrateSubcommand::new(idx, speed));
                }
            }
            VibrateCommand::SpeedVec(vec) => {
                if vec.len() as u32 > vibrator_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} vibrators, but {} commands were sent.",
                            vibrator_count,
                            vec.len()
                        ),
                    )));
                }
                speed_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    speed_vec.push(VibrateSubcommand::new(i as u32, *v));
                }
            }
        }
        let msg = ButtplugMessageUnion::VibrateCmd(VibrateCmd::new(self.index, speed_vec));
        self.send_message_expect_ok(msg).await
    }

    pub async fn linear(&mut self, linear_cmd: LinearCommand) -> ButtplugClientResult {
        if !self.allowed_messages.contains_key("LinearCmd") {
            return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                "Device does not support linear movement.",
            )));
        }
        let mut linear_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get("LinearCmd") {
            if let Some(v) = features.feature_count {
                linear_count = v;
            }
        }
        let mut linear_vec: Vec<VectorSubcommand>;
        match linear_cmd {
            LinearCommand::Linear(dur, pos) => {
                linear_vec = Vec::with_capacity(linear_count as usize);
                for i in 0..linear_count {
                    linear_vec.push(VectorSubcommand::new(i, dur, pos));
                }
            }
            LinearCommand::LinearMap(map) => {
                if map.len() as u32 > linear_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} linear actuators, but {} commands were sent.",
                            linear_count,
                            map.len()
                        ),
                    )));
                }
                linear_vec = Vec::with_capacity(map.len() as usize);
                for (idx, (dur, pos)) in map {
                    if idx > linear_count - 1 {
                        return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                            &format!(
                                "Max linear index is {}, command referenced {}.",
                                linear_count, idx
                            ),
                        )));
                    }
                    linear_vec.push(VectorSubcommand::new(idx, dur, pos));
                }
            }
            LinearCommand::LinearVec(vec) => {
                if vec.len() as u32 > linear_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} linear actuators, but {} commands were sent.",
                            linear_count,
                            vec.len()
                        ),
                    )));
                }
                linear_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    linear_vec.push(VectorSubcommand::new(i as u32, v.0, v.1));
                }
            }
        }
        let msg = ButtplugMessageUnion::LinearCmd(LinearCmd::new(self.index, linear_vec));
        self.send_message_expect_ok(msg).await
    }

    pub async fn rotate(&mut self, rotate_cmd: RotateCommand) -> ButtplugClientResult {
        if !self.allowed_messages.contains_key("RotateCmd") {
            return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                "Device does not support rotation.",
            )));
        }
        let mut rotate_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get("RotateCmd") {
            if let Some(v) = features.feature_count {
                rotate_count = v;
            }
        }
        let mut rotate_vec: Vec<RotationSubcommand>;
        match rotate_cmd {
            RotateCommand::Rotate(speed, clockwise) => {
                rotate_vec = Vec::with_capacity(rotate_count as usize);
                for i in 0..rotate_count {
                    rotate_vec.push(RotationSubcommand::new(i, speed, clockwise));
                }
            }
            RotateCommand::RotateMap(map) => {
                if map.len() as u32 > rotate_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} rotators, but {} commands were sent.",
                            rotate_count,
                            map.len()
                        ),
                    )));
                }
                rotate_vec = Vec::with_capacity(map.len() as usize);
                for (idx, (speed, clockwise)) in map {
                    if idx > rotate_count - 1 {
                        return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                            &format!(
                                "Max rotate index is {}, command referenced {}.",
                                rotate_count, idx
                            ),
                        )));
                    }
                    rotate_vec.push(RotationSubcommand::new(idx, speed, clockwise));
                }
            }
            RotateCommand::RotateVec(vec) => {
                if vec.len() as u32 > rotate_count {
                    return Err(ButtplugClientError::from(ButtplugDeviceError::new(
                        &format!(
                            "Device only has {} rotators, but {} commands were sent.",
                            rotate_count,
                            vec.len()
                        ),
                    )));
                }
                rotate_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    rotate_vec.push(RotationSubcommand::new(i as u32, v.0, v.1));
                }
            }
        }
        let msg = ButtplugMessageUnion::RotateCmd(RotateCmd::new(self.index, rotate_vec));
        self.send_message_expect_ok(msg).await
    }

    pub async fn stop(&mut self) -> ButtplugClientResult {
        // All devices accept StopDeviceCmd
        self.send_message_expect_ok(ButtplugMessageUnion::StopDeviceCmd(StopDeviceCmd::default()))
            .await
    }
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
