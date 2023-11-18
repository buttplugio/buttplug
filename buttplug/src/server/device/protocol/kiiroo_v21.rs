// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::fleshlight_launch_helper::calculate_speed;
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
      HardwareCommand,
      HardwareEvent,
      HardwareReadCmd,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
    protocol::{generic_protocol_setup, ProtocolHandler},
  },
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use dashmap::DashSet;
use futures::{
  future::{self, BoxFuture},
  FutureExt,
  StreamExt,
};
use std::{
  default::Default,
  pin::Pin,
  sync::{
    atomic::{AtomicU8, Ordering::SeqCst},
    Arc,
  },
};
use tokio::sync::broadcast;

generic_protocol_setup!(KiirooV21, "kiiroo-v21");

pub struct KiirooV21 {
  previous_position: Arc<AtomicU8>,
  // Set of sensors we've subscribed to for updates.
  subscribed_sensors: Arc<DashSet<u32>>,
  event_stream: broadcast::Sender<ButtplugServerDeviceMessage>,
}

impl Default for KiirooV21 {
  fn default() -> Self {
    let (sender, _) = broadcast::channel(256);
    Self {
      previous_position: Default::default(),
      subscribed_sensors: Arc::new(DashSet::new()),
      event_stream: sender,
    }
  }
}

impl ProtocolHandler for KiirooV21 {
  fn handle_scalar_vibrate_cmd(
    &self,
    _: u32,
    scalar: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      vec![0x01, scalar as u8],
      false,
    )
    .into()])
  }

  fn handle_linear_cmd(
    &self,
    message: message::LinearCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let v = message.vectors()[0].clone();
    // In the protocol, we know max speed is 99, so convert here. We have to
    // use AtomicU8 because there's no AtomicF64 yet.
    let previous_position = self.previous_position.load(SeqCst);
    let distance = (previous_position as f64 - (v.position() * 99f64)).abs() / 99f64;
    let fl_cmd = message::FleshlightLaunchFW12Cmd::new(
      message.device_index(),
      (v.position() * 99f64) as u8,
      (calculate_speed(distance, v.duration()) * 99f64) as u8,
    );
    self.handle_fleshlight_launch_fw12_cmd(fl_cmd)
  }

  fn handle_fleshlight_launch_fw12_cmd(
    &self,
    message: message::FleshlightLaunchFW12Cmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let previous_position = self.previous_position.clone();
    let position = message.position();
    previous_position.store(position, SeqCst);
    Ok(vec![HardwareWriteCmd::new(
      Endpoint::Tx,
      [0x03, 0x00, message.speed(), message.position()].to_vec(),
      false,
    )
    .into()])
  }

  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    debug!("Trying to get battery reading.");
    // Reading the "whitelist" endpoint for this device retrieves the battery level,
    // which is byte 5. All other bytes of the 20-byte result are unknown.
    let msg = HardwareReadCmd::new(Endpoint::Whitelist, 20, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      let data = hw_msg.data();
      if data.len() != 20 {
        // Maybe not the Kiiroo Pearl 2.1?
        return Err(ButtplugDeviceError::DeviceCommunicationError(
          "Kiiroo battery data not expected length!".to_owned(),
        ));
      }
      let battery_level = data[5] as i32;
      let battery_reading = message::SensorReading::new(
        message.device_index(),
        *message.sensor_index(),
        *message.sensor_type(),
        vec![battery_level],
      );
      debug!("Got battery reading: {}", battery_level);
      Ok(battery_reading.into())
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
    if self.subscribed_sensors.contains(message.sensor_index()) {
      return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    // Format for the Kiiroo Pearl 2.1:
    // Byte 0-1: Raw u16be pressure sensor, smaller values indicate more pressure, channel 1.
    //           Zero values differ even between sensors on same device.
    //           Legal range is not known (might even be i16le),
    //           actual range on one device is around 850±50.
    // Byte 2-3: Same, channel 2.
    // Byte 4-5: Same, channel 3.
    // Byte 6-7: Same, channel 4.
    // Byte 8: Flags corresponding to pressure regions, thresholded on device:
    //         LSB is channel 1 pressed, next least significant bit is channel 2, etc.
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      if sensors.is_empty() {
        device
          .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
          .await?;
        let sender = self.event_stream.clone();
        let mut hardware_stream = device.event_stream();
        let stream_sensors = sensors.clone();
        let device_index = message.device_index();
        // If we subscribe successfully, we need to set up our event handler.
        async_manager::spawn(async move {
          while let Ok(info) = hardware_stream.recv().await {
            // If we have no receivers, quit.
            if sender.receiver_count() == 0 || stream_sensors.is_empty() {
              return;
            }
            if let HardwareEvent::Notification(_, endpoint, data) = info {
              if endpoint == Endpoint::Rx {
                if data.len() != 9 {
                  // Maybe not the Kiiroo Pearl 2.1?
                  error!("Kiiroo sensor data not expected length!");
                  continue;
                }
                // Extract our pressure values.
                // Invert analog values so that the value increases with pressure.
                let analog: Vec<i32> = (0..4)
                  .into_iter()
                  .map(|i| {
                    (u16::MAX as i32) - ((data[2 * i] as i32) << 8 | (data[2 * i + 1] as i32))
                  })
                  .collect();
                let digital: Vec<i32> = (0..4)
                  .into_iter()
                  .map(|i| ((data[8] as i32) >> i) & 1)
                  .collect();
                for ((sensor_index, sensor_type), sensor_data) in (0u32..)
                  .zip([SensorType::Pressure, SensorType::Button])
                  .zip([analog, digital])
                {
                  if stream_sensors.contains(&sensor_index)
                    && sender
                      .send(
                        SensorReading::new(device_index, sensor_index, sensor_type, sensor_data)
                          .into(),
                      )
                      .is_err()
                  {
                    debug!(
                    "Hardware device listener for Kiiroo 2.1 device shut down, returning from task."
                  );
                    return;
                  }
                }
              }
            }
          }
        });
      }
      sensors.insert(*message.sensor_index());
      Ok(message::Ok::new(message.id()).into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: message::SensorUnsubscribeCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    if !self.subscribed_sensors.contains(message.sensor_index()) {
      return future::ready(Ok(message::Ok::new(message.id()).into())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to end our BLE
      // characteristic subscription.
      sensors.remove(message.sensor_index());
      if sensors.is_empty() {
        device
          .unsubscribe(&HardwareUnsubscribeCmd::new(Endpoint::Rx))
          .await?;
      }
      Ok(message::Ok::new(message.id()).into())
    }
    .boxed()
  }
}
