// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2019 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Representation and management of devices connected to the server.

use super::{
    connectors::ButtplugClientConnectorError, internal::ButtplugClientDeviceEvent,
    ButtplugClientError, ButtplugClientResult,
};
use crate::{
    core::{
        errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
        messages::{
            ButtplugMessageUnion, DeviceAdded, DeviceMessageInfo, LinearCmd, ButtplugDeviceMessageType,
            RotateCmd, RotationSubcommand, StopDeviceCmd, VectorSubcommand, VibrateCmd,
            VibrateSubcommand, MessageAttributesMap
        },
    },
    util::future::{ButtplugMessageFuture, ButtplugMessageFuturePair},
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
    pub allowed_messages: MessageAttributesMap,
    message_sender: Sender<ButtplugMessageFuturePair>,
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
        allowed_messages: MessageAttributesMap,
        message_sender: Sender<ButtplugMessageFuturePair>,
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
        let fut = ButtplugMessageFuture::default();
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
                ButtplugMessageError {
                    message: "Got unexpected message type.".to_owned(),
                }
                .into(),
            )),
        }
    }

    async fn check_for_events(&mut self) -> ButtplugClientResult {
        if !self.client_connected {
            return Err(ButtplugClientConnectorError::new("Client not connected.").into());
        }
        if !self.device_connected {
            return Err(ButtplugDeviceError::new("Device not connected.").into());
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
                    return Err(ButtplugClientConnectorError::new("Client not connected.").into());
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
            return Err(ButtplugClientConnectorError::new("Client not connected.").into());
        }
        if !self.device_connected {
            return Err(ButtplugDeviceError::new("Device not connected.").into());
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
        if !self.allowed_messages.contains_key(&ButtplugDeviceMessageType::VibrateCmd) {
            return Err(ButtplugDeviceError::new("Device does not support vibration.").into());
        }
        let mut vibrator_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get(&ButtplugDeviceMessageType::VibrateCmd) {
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
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} vibrators, but {} commands were sent.",
                        vibrator_count,
                        map.len()
                    ))
                    .into());
                }
                speed_vec = Vec::with_capacity(map.len() as usize);
                for (idx, speed) in map {
                    if idx > vibrator_count - 1 {
                        return Err(ButtplugDeviceError::new(&format!(
                            "Max vibrator index is {}, command referenced {}.",
                            vibrator_count, idx
                        ))
                        .into());
                    }
                    speed_vec.push(VibrateSubcommand::new(idx, speed));
                }
            }
            VibrateCommand::SpeedVec(vec) => {
                if vec.len() as u32 > vibrator_count {
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} vibrators, but {} commands were sent.",
                        vibrator_count,
                        vec.len()
                    ))
                    .into());
                }
                speed_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    speed_vec.push(VibrateSubcommand::new(i as u32, *v));
                }
            }
        }
        let msg = VibrateCmd::new(self.index, speed_vec).into();
        self.send_message_expect_ok(msg).await
    }

    pub async fn linear(&mut self, linear_cmd: LinearCommand) -> ButtplugClientResult {
        if !self.allowed_messages.contains_key(&ButtplugDeviceMessageType::LinearCmd) {
            return Err(
                ButtplugDeviceError::new("Device does not support linear movement.").into(),
            );
        }
        let mut linear_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get(&ButtplugDeviceMessageType::LinearCmd) {
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
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} linear actuators, but {} commands were sent.",
                        linear_count,
                        map.len()
                    ))
                    .into());
                }
                linear_vec = Vec::with_capacity(map.len() as usize);
                for (idx, (dur, pos)) in map {
                    if idx > linear_count - 1 {
                        return Err(ButtplugDeviceError::new(&format!(
                            "Max linear index is {}, command referenced {}.",
                            linear_count, idx
                        ))
                        .into());
                    }
                    linear_vec.push(VectorSubcommand::new(idx, dur, pos));
                }
            }
            LinearCommand::LinearVec(vec) => {
                if vec.len() as u32 > linear_count {
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} linear actuators, but {} commands were sent.",
                        linear_count,
                        vec.len()
                    ))
                    .into());
                }
                linear_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    linear_vec.push(VectorSubcommand::new(i as u32, v.0, v.1));
                }
            }
        }
        let msg = LinearCmd::new(self.index, linear_vec).into();
        self.send_message_expect_ok(msg).await
    }

    pub async fn rotate(&mut self, rotate_cmd: RotateCommand) -> ButtplugClientResult {
        if !self.allowed_messages.contains_key(&ButtplugDeviceMessageType::RotateCmd) {
            return Err(ButtplugDeviceError::new("Device does not support rotation.").into());
        }
        let mut rotate_count: u32 = 0;
        if let Some(features) = self.allowed_messages.get(&ButtplugDeviceMessageType::RotateCmd) {
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
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} rotators, but {} commands were sent.",
                        rotate_count,
                        map.len()
                    ))
                    .into());
                }
                rotate_vec = Vec::with_capacity(map.len() as usize);
                for (idx, (speed, clockwise)) in map {
                    if idx > rotate_count - 1 {
                        return Err(ButtplugDeviceError::new(&format!(
                            "Max rotate index is {}, command referenced {}.",
                            rotate_count, idx
                        ))
                        .into());
                    }
                    rotate_vec.push(RotationSubcommand::new(idx, speed, clockwise));
                }
            }
            RotateCommand::RotateVec(vec) => {
                if vec.len() as u32 > rotate_count {
                    return Err(ButtplugDeviceError::new(&format!(
                        "Device only has {} rotators, but {} commands were sent.",
                        rotate_count,
                        vec.len()
                    ))
                    .into());
                }
                rotate_vec = Vec::with_capacity(vec.len() as usize);
                for (i, v) in vec.iter().enumerate() {
                    rotate_vec.push(RotationSubcommand::new(i as u32, v.0, v.1));
                }
            }
        }
        let msg = RotateCmd::new(self.index, rotate_vec).into();
        self.send_message_expect_ok(msg).await
    }

    pub async fn stop(&mut self) -> ButtplugClientResult {
        // All devices accept StopDeviceCmd
        self.send_message_expect_ok(StopDeviceCmd::default().into())
            .await
    }
}

impl
    From<(
        &DeviceAdded,
        Sender<ButtplugMessageFuturePair>,
        Receiver<ButtplugClientDeviceEvent>,
    )> for ButtplugClientDevice
{
    fn from(
        msg_sender_tuple: (
            &DeviceAdded,
            Sender<ButtplugMessageFuturePair>,
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
        Sender<ButtplugMessageFuturePair>,
        Receiver<ButtplugClientDeviceEvent>,
    )> for ButtplugClientDevice
{
    fn from(
        msg_sender_tuple: (
            &DeviceMessageInfo,
            Sender<ButtplugMessageFuturePair>,
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
