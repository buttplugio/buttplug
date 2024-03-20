// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
    core::{
        errors::ButtplugDeviceError,
        message::{
            self,
            ButtplugDeviceMessage,
            ButtplugMessage,
            ButtplugServerDeviceMessage,
            ButtplugServerMessage,
            Endpoint,
            SensorReading,
            SensorType,
        },
    },
    server::device::{
        hardware::{
            Hardware,
            HardwareEvent,
            HardwareSubscribeCmd,
            HardwareUnsubscribeCmd,
        },
        protocol::{generic_protocol_setup, ProtocolHandler},
    },
    util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use futures::{
    future::{self, BoxFuture},
    FutureExt,
    StreamExt,
};
use std::{
    default::Default,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Arc,
    },
};
use tokio::sync::broadcast;

generic_protocol_setup!(AdamHealth, "adamhealth");

pub struct AdamHealth {
    // Set of sensors we've subscribed to for updates.
    subscribed: Arc<AtomicBool>,
    event_stream: broadcast::Sender<ButtplugServerDeviceMessage>,
}

impl Default for AdamHealth {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            subscribed: Arc::new(AtomicBool::new(false)),
            event_stream: sender,
        }
    }
}

#[derive(Debug)]
enum AdamDataTag {
    UNKNOWN,
    PRESSURE,
    BATTERY,
}

impl ProtocolHandler for AdamHealth {
    fn handle_battery_level_cmd(
        &self,
        device: Arc<Hardware>,
        message: message::SensorReadCmd,
    ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
        self.handle_sensor_read_cmd(device, message)
    }

    fn handle_sensor_read_cmd(
        &self,
        device: Arc<Hardware>,
        message: message::SensorReadCmd,
    ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
        let sensor_type = message.sensor_type().clone();
        let request = message::SensorSubscribeCmd::new(
            message.device_index(), 0, sensor_type
        );
        let mut incoming = self.event_stream();
        let fut = self.handle_sensor_subscribe_cmd(device, request);
        async move {
            let _ = fut.await?;
            let mut result = Err(ButtplugDeviceError::DeviceConnectionError("Connection error".to_string()));
            while let Some(data) = incoming.next().await {
                if let ButtplugServerDeviceMessage::SensorReading(reading) = data {
                    if reading.sensor_type() == sensor_type {
                        result = Ok(reading.into());
                        break
                    }
                }
            }
            result
        }
            .boxed()
    }

    fn event_stream(
        &self,
    ) -> Pin<Box<dyn futures::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
        convert_broadcast_receiver_to_stream(self.event_stream.subscribe()).boxed()
    }

    fn handle_sensor_subscribe_cmd(
        &self,
        device: Arc<Hardware>,
        message: message::SensorSubscribeCmd,
    ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
        if self.subscribed.load(SeqCst) {
            return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
        }
        // The AdamHealth sensor has a single characteristic that streams interleaved data
        // There will be a payload identifier code, and then the data
        // and then a different identifier code, and then that data
        let subscribed = self.subscribed.clone();
        async move {
            // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
            // characteristic subscription.
            if !subscribed.load(SeqCst) {
                device
                    .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
                    .await?;
                let sender = self.event_stream.clone();
                let mut hardware_stream = device.event_stream();
                let keep_looping = subscribed.clone();
                let device_index = message.device_index();
                // If we subscribe successfully, we need to set up our event handler.
                let _ = async_manager::spawn(async move {
                    let mut data_tag = AdamDataTag::UNKNOWN;
                    while let Ok(info) = hardware_stream.recv().await {
                        // If we have no receivers, quit.
                        // receiver_count will always include 1 for our sender
                        if sender.receiver_count() <= 1 || !keep_looping.load(SeqCst) {
                            debug!("No active listeners for AdamHealth sensor, unsubscribing and returning from task.");
                            // todo factor out to handle_sensor_unsubscribe_cmd
                            keep_looping.store(false, SeqCst);
                            let _ = device
                                .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Rx))
                                .await;
                            return;
                        }
                        if let HardwareEvent::Notification(_, endpoint, data) = info {
                            if endpoint == Endpoint::Rx {
                                if data == "1300".as_bytes() { // incoming sensor value
                                    data_tag = AdamDataTag::PRESSURE
                                } else if data == "1301".as_bytes() { // incoming battery value
                                    data_tag = AdamDataTag::BATTERY
                                } else {
                                    // unhandled dataTag, or sensor measurement
                                    let value = std::str::from_utf8(data.as_slice()).unwrap_or("").parse::<i32>();
                                    if matches!(data_tag, AdamDataTag::PRESSURE) && value.is_ok() {
                                        let sensor_value = value.unwrap().clamp(0, 500);
                                        let result = sender.send(
                                            SensorReading::new(device_index, 0, SensorType::Pressure, vec![sensor_value]).into(),
                                        );
                                        if result.is_err() {
                                            debug!("Hardware device listener for AdamHealth sensor shut down, returning from task.");
                                            return;
                                        }
                                    } else if matches!(data_tag, AdamDataTag::BATTERY) && value.is_ok() { // incoming battery value
                                        let sensor_value = value.unwrap().clamp(0, 100);
                                        let result = sender.send(
                                            SensorReading::new(device_index, 0, SensorType::Battery, vec![sensor_value]).into(),
                                        );
                                        if result.is_err() {
                                            debug!("Hardware device listener for AdamHealth sensor shut down, returning from task.");
                                            return;
                                        }
                                    }
                                    data_tag = AdamDataTag::UNKNOWN;
                                }
                            }
                        }
                    }
                });
            }
            subscribed.store(true, SeqCst);
            Ok(message::Ok::new(message.id()).into())
        }
            .boxed()
    }

    fn handle_sensor_unsubscribe_cmd(
        &self,
        device: Arc<Hardware>,
        message: message::SensorUnsubscribeCmd,
    ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
        if !self.subscribed.load(SeqCst) {
            return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
        }
        async move {
            // If we have no sensors we're currently subscribed to, we'll need to end our BLE
            // characteristic subscription.
            self.subscribed.store(false, SeqCst);
            device
                .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Rx))
                .await?;
            Ok(message::Ok::new(message.id()).into())
        }
            .boxed()
    }
}
