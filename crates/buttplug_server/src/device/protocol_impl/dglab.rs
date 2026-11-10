// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use futures::{
  FutureExt,
  StreamExt,
  future::{self, BoxFuture},
};
use std::pin::Pin;
use std::sync::{
  Arc,
  atomic::{AtomicU8, Ordering},
};
use std::time::Duration;
use tokio::sync::broadcast::{self, error::RecvError};
use uuid::{Uuid, uuid};

use crate::{
  device::{
    hardware::{
      Hardware,
      HardwareCommand,
      HardwareEvent,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
      HardwareWriteCmd,
    },
    protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
  },
  generic_protocol_initializer_setup,
  message::ButtplugServerDeviceMessage,
};
use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{InputReadingV4, InputType, InputValue},
  util::{async_manager, stream::convert_broadcast_receiver_to_stream},
};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};

const DGLAB_PROTOCOL_UUID: Uuid = uuid!("3621e028-1af9-4bb3-8524-bded1fa5dc6a");
const DGLAB_DISPALY_UUID: Uuid = uuid!("70b44cf0-d77c-4b4f-8427-07ee5d5409f9");
const DGLAB_WAVEFORM_UUID: Uuid = uuid!("77a75680-92f2-4fda-b2b0-c5599384e660");
const DGLAB_WAVEFORM_INTERVAL: u64 = 100;

generic_protocol_initializer_setup!(DGLab, "dglab");

async fn dglab_waveform(device: Arc<Hardware>) {
  loop {
    if device
      .write_value(&HardwareWriteCmd::new(
        &[DGLAB_WAVEFORM_UUID],
        Endpoint::Tx,
        vec![
          0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x64, 0x64, 0x64, 0x00, 0x00, 0x00,
          0x00, 0x64, 0x64, 0x64, 0x64,
        ],
        false,
      ))
      .await
      .is_err()
    {
      return;
    }
    async_manager::sleep(Duration::from_millis(DGLAB_WAVEFORM_INTERVAL)).await;
  }
}

#[derive(Default)]
pub struct DGLabInitializer {}

#[async_trait]
impl ProtocolInitializer for DGLabInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let has_pressure = device_definition
      .features()
      .values()
      .any(|x| x.contains_input(InputType::Pressure));
    let msg = HardwareWriteCmd::new(
      &[DGLAB_PROTOCOL_UUID],
      Endpoint::Tx,
      vec![0x50, 0x01, if has_pressure { 0xd0 } else { 0x00 }],
      false,
    );
    hardware.write_value(&msg).await?;

    buttplug_core::spawn!("DGLabWaveform", dglab_waveform(hardware.clone(),));
    Ok(Arc::new(DGLab::default()))
  }
}

pub struct DGLab {
  speeds: [AtomicU8; 2],
  // Set of sensors we've subscribed to for updates.
  subscribed_sensors: Arc<AtomicU8>,
  event_stream: broadcast::Sender<ButtplugServerDeviceMessage>,
}

impl Default for DGLab {
  fn default() -> Self {
    let (sender, _) = broadcast::channel(256);
    Self {
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
      subscribed_sensors: Arc::new(AtomicU8::new(0)),
      event_stream: sender,
    }
  }
}

impl ProtocolHandler for DGLab {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);

    let s1 = self.speeds[0].load(Ordering::Relaxed);
    let s2 = self.speeds[1].load(Ordering::Relaxed);

    Ok(vec![
      HardwareWriteCmd::new(
        &[DGLAB_PROTOCOL_UUID],
        Endpoint::Tx,
        vec![0xb3, s1, s2],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &[DGLAB_DISPALY_UUID],
        Endpoint::Tx,
        vec![
          0xb2, 0xFF, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
          0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x08, 0x09, s1, s2,
        ],
        false,
      )
      .into(),
    ])
  }

  fn handle_input_subscribe_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    let sensors = self.subscribed_sensors.load(Ordering::Relaxed);
    if (sensors & (1 << feature_index as u8)) > 0 {
      debug!("DGLab already subscribed, skipping");
      return future::ready(Ok(())).boxed();
    }
    // Readout value: 0xD001xxxxxxxxxxxx1603xxxxxxxxxx
    // Byte 7-8: i16 pressure in kPa * 100 (we don't do decimal, so * 10 to get Pa)
    let stream_sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      if sensors == 0 {
        device
          .subscribe(&HardwareSubscribeCmd::new(feature_id, Endpoint::Rx))
          .await?;
        let sender = self.event_stream.clone();
        let mut hardware_stream = device.event_stream();
        let stream_sensors = stream_sensors.clone();
        debug!("Starting DGLab subscription");
        // If we subscribe successfully, we need to set up our event handler.
        buttplug_core::spawn!("DGLab subscription event handler", async move {
          let mut cached_value = 0i32;
          loop {
            let info = match hardware_stream.recv().await {
              Ok(info) => info,
              Err(RecvError::Lagged(n)) => {
                warn!("DGLab subscription lagged, dropped {} messages", n);
                continue;
              }
              Err(RecvError::Closed) => break,
            };
            let subscribed_sensors = stream_sensors.load(Ordering::Relaxed);
            // If we have no receivers, quit.
            if sender.receiver_count() == 0 || subscribed_sensors == 0 {
              return;
            }
            if let HardwareEvent::Notification(_, endpoint, data) = info
              && endpoint == Endpoint::Rx
            {
              if data.len() < 9 {
                // Not even sure how this would happen, error and continue on.
                error!("DGLab data not expected length!");
                continue;
              }
              // Extract our two pressure values.
              let presure: i32 = i16::from_le_bytes([data[8], data[9]]) as i32 * 10;
              debug!("DGLab Reading {} {}", subscribed_sensors, presure);
              if (subscribed_sensors & (1 << 0)) > 0
                && cached_value != presure
                && sender
                  .send(
                    InputReadingV4::new(
                      device_index,
                      0,
                      buttplug_core::message::InputTypeReading::Pressure(InputValue::new(presure)),
                    )
                    .into(),
                  )
                  .is_err()
              {
                debug!("Hardware device listener for DGLab Boost shut down, returning from task.");
                return;
              }
              cached_value = presure;
            }
          }
        });
      }
      stream_sensors.store(
        stream_sensors.load(Ordering::Relaxed) | (1 << feature_index),
        Ordering::Relaxed,
      );
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
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    let sensors = self.subscribed_sensors.load(Ordering::Relaxed);
    if (sensors & (1 << feature_index as u8)) == 0 {
      return future::ready(Ok(())).boxed();
    }
    let sensors = self.subscribed_sensors.clone();
    async move {
      // If we have no sensors we're currently subscribed to, we'll need to bring up our BLE
      // characteristic subscription.
      info!("Sensor before: {}", sensors.load(Ordering::Relaxed));
      sensors.store(
        sensors.load(Ordering::Relaxed) & !(1 << feature_index),
        Ordering::Relaxed,
      );
      info!("Sensor after: {}", sensors.load(Ordering::Relaxed));
      if sensors.load(Ordering::Relaxed) == 0 {
        device
          .unsubscribe(&HardwareUnsubscribeCmd::new(feature_id, Endpoint::Rx))
          .await?;
      }
      Ok(())
    }
    .boxed()
  }

  fn event_stream(
    &self,
  ) -> Pin<Box<dyn futures::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    convert_broadcast_receiver_to_stream(self.event_stream.subscribe()).boxed()
  }
}
