// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  device::{
    hardware::{Hardware, HardwareEvent, HardwareSubscribeCmd, HardwareUnsubscribeCmd},
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
  message::ButtplugServerDeviceMessage,
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{InputData, InputReadingV4, InputType},
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use buttplug_server_device_config::Endpoint;
use dashmap::DashSet;
use futures::{
  future::{self, BoxFuture},
  FutureExt,
  StreamExt,
};
use std::{pin::Pin, sync::Arc};
use tokio::sync::broadcast;
use uuid::Uuid;

generic_protocol_setup!(KGoalBoost, "kgoal-boost");

pub struct KGoalBoost {
  // Set of sensors we've subscribed to for updates.
  subscribed_sensors: Arc<DashSet<u32>>,
  event_stream: broadcast::Sender<ButtplugServerDeviceMessage>,
}

impl Default for KGoalBoost {
  fn default() -> Self {
    let (sender, _) = broadcast::channel(256);
    Self {
      subscribed_sensors: Arc::new(DashSet::new()),
      event_stream: sender,
    }
  }
}

impl ProtocolHandler for KGoalBoost {
  fn event_stream(
    &self,
  ) -> Pin<Box<dyn futures::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    convert_broadcast_receiver_to_stream(self.event_stream.subscribe()).boxed()
  }

  fn handle_input_subscribe_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    if self.subscribed_sensors.contains(&feature_index) {
      return future::ready(Ok(())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    // Readout value: 0x000104000005d3
    // Byte 0: Always 0x00
    // Byte 1: Always 0x01
    // Byte 2: Always 0x04
    // Byte 3-4: Normalized u16 Reading
    // Byte 5-6: Raw u16 Reading
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      if sensors.is_empty() {
        device
          .subscribe(&HardwareSubscribeCmd::new(feature_id, Endpoint::RxPressure))
          .await?;
        let sender = self.event_stream.clone();
        let mut hardware_stream = device.event_stream();
        let stream_sensors = sensors.clone();
        // If we subscribe successfully, we need to set up our event handler.
        async_manager::spawn(async move {
          while let Ok(info) = hardware_stream.recv().await {
            // If we have no receivers, quit.
            if sender.receiver_count() == 0 || stream_sensors.is_empty() {
              return;
            }
            if let HardwareEvent::Notification(_, endpoint, data) = info {
              if endpoint == Endpoint::RxPressure {
                if data.len() < 7 {
                  // Not even sure how this would happen, error and continue on.
                  error!("KGoal Boost data not expected length!");
                  continue;
                }
                // Extract our two pressure values.
                let normalized = (data[3] as u32) << 8 | data[4] as u32;
                let unnormalized = (data[5] as u32) << 8 | data[6] as u32;
                if stream_sensors.contains(&0)
                  && sender
                    .send(
                      InputReadingV4::new(
                        device_index,
                        feature_index,
                        buttplug_core::message::InputTypeData::Pressure(InputData::new(normalized))
                      )
                      .into(),
                    )
                    .is_err()
                {
                  debug!(
                    "Hardware device listener for KGoal Boost shut down, returning from task."
                  );
                  return;
                }
                if stream_sensors.contains(&1)
                  && sender
                    .send(
                      InputReadingV4::new(
                        device_index,
                        feature_index,
                        buttplug_core::message::InputTypeData::Pressure(InputData::new(unnormalized))
                      )
                      .into(),
                    )
                    .is_err()
                {
                  debug!(
                    "Hardware device listener for KGoal Boost shut down, returning from task."
                  );
                  return;
                }
              }
            }
          }
        });
      }
      sensors.insert(feature_index);
      Ok(())
    }
    .boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    if !self.subscribed_sensors.contains(&feature_index) {
      return future::ready(Ok(())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      sensors.remove(&feature_index);
      if sensors.is_empty() {
        device
          .unsubscribe(&HardwareUnsubscribeCmd::new(
            feature_id,
            Endpoint::RxPressure,
          ))
          .await?;
      }
      Ok(())
    }
    .boxed()
  }
}
