// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use uuid::Uuid;

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{generic_protocol_setup, ProtocolHandler},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

generic_protocol_setup!(Cupido, "cupido");

#[derive(Default)]
pub struct Cupido {}

impl ProtocolHandler for Cupido {
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![HardwareWriteCmd::new(
      &[feature_id],
      Endpoint::Tx,
      vec![0xb0, 0x03, 0, 0, 0, speed as u8, 0xaa],
      false,
    )
    .into()])
  }

  /* -- expensive if we're not caching the battery
  fn handle_battery_level_cmd(
    &self,
    device: Arc<Hardware>,
    message: SensorReadCmd,
  ) -> BoxFuture<Result<ButtplugServerMessage, ButtplugDeviceError>> {
    let mut device_notification_receiver = device.event_stream();
    async move {
      device
        .subscribe(&HardwareSubscribeCmd::new(Endpoint::Rx))
        .await?;
      while let Ok(event) = device_notification_receiver.recv().await {
        return match event {
          HardwareEvent::Notification(_, endpoint, data) => {
            if endpoint != Endpoint::Rx {
              continue;
            }
            if data.len() == 6 && data[0..5] != vec![0xb0, 0, 0, 0, 1] {
              continue;
            }
            let battery_reading = SensorReading::new(
              message.device_index(),
              *message.sensor_index(),
              *message.input_type(),
              vec![data[5] as i32],
            );
            Ok(battery_reading.into())
          }
          HardwareEvent::Disconnected(_) => Err(ButtplugDeviceError::ProtocolSpecificError(
            "Cupido".to_owned(),
            "Cupido Device disconnected while getting Battery info.".to_owned(),
          )),
        };
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "Cupido".to_owned(),
        "Cupido Device disconnected while getting Battery info.".to_owned(),
      ))
    }
    .boxed()
  }*/
}
