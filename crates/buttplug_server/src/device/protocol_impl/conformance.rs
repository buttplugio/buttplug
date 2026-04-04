// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::Arc;
use uuid::Uuid;

use crate::device::{
  hardware::{
    Hardware, HardwareCommand, HardwareReadCmd, HardwareSubscribeCmd, HardwareUnsubscribeCmd,
    HardwareWriteCmd,
  },
  protocol::{ProtocolHandler, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_core::message::{InputReadingV4, InputType, InputValue};
use buttplug_server_device_config::Endpoint;
use futures::{FutureExt, future::BoxFuture};

generic_protocol_setup!(Conformance, "conformance");

#[derive(Default)]
pub struct Conformance {}

impl ProtocolHandler for Conformance {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = speed as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_rotate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let bytes = speed.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = speed as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_spray_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = level as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_constrict_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = level as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_temperature_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let bytes = level.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_led_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = level as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_output_position_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let value = position as i32;
    let bytes = value.to_le_bytes().to_vec();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [vec![feature_index as u8], bytes].concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_hw_position_with_duration_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    position: u32,
    duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let pos_bytes = (position as i32).to_le_bytes();
    let dur_bytes = (duration as i32).to_le_bytes();
    Ok(vec![
      HardwareWriteCmd::new(
        &[Uuid::nil()],
        Endpoint::Tx,
        [
          vec![feature_index as u8],
          pos_bytes.to_vec(),
          dur_bytes.to_vec(),
        ]
        .concat(),
        false,
      )
      .into(),
    ])
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<InputReadingV4, ButtplugDeviceError>> {
    let endpoint = match sensor_type {
      InputType::Battery => Endpoint::RxBLEBattery,
      InputType::Rssi => Endpoint::Generic0,
      InputType::Button => Endpoint::Generic1,
      InputType::Pressure => Endpoint::Generic2,
      InputType::Unknown | InputType::Depth | InputType::Position => Endpoint::Generic3,
    };

    let msg = HardwareReadCmd::new(feature_id, endpoint, 1, 0);
    let fut = device.read_value(&msg);
    async move {
      let hw_msg = fut.await?;
      // Convert the hardware reading into an InputReadingV4
      // Extract the value from the hardware reading data
      let value = if !hw_msg.data().is_empty() {
        hw_msg.data()[0] as i32
      } else {
        0
      };

      // Build the reading based on sensor type
      let input_type_reading = match sensor_type {
        InputType::Battery => {
          buttplug_core::message::InputTypeReading::Battery(InputValue::new(value as u8))
        }
        InputType::Rssi => {
          buttplug_core::message::InputTypeReading::Rssi(InputValue::new(value as i8))
        }
        InputType::Button => {
          buttplug_core::message::InputTypeReading::Button(InputValue::new(value as u8))
        }
        InputType::Pressure => {
          buttplug_core::message::InputTypeReading::Pressure(InputValue::new(value as u32))
        }
        InputType::Depth | InputType::Position | InputType::Unknown => {
          return Err(ButtplugDeviceError::UnhandledCommand(format!(
            "Sensor type not supported: {:?}",
            sensor_type
          )));
        }
      };

      Ok(InputReadingV4::new(
        device_index,
        feature_index,
        input_type_reading,
      ))
    }
    .boxed()
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device_index: u32,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    let endpoint = match sensor_type {
      InputType::Battery => Endpoint::RxBLEBattery,
      InputType::Rssi => Endpoint::Generic0,
      InputType::Button => Endpoint::Generic1,
      InputType::Pressure => Endpoint::Generic2,
      InputType::Unknown | InputType::Depth | InputType::Position => Endpoint::Generic3,
    };

    let msg = HardwareSubscribeCmd::new(feature_id, endpoint);
    async move { device.subscribe(&msg).await }.boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'_, Result<(), ButtplugDeviceError>> {
    let endpoint = match sensor_type {
      InputType::Battery => Endpoint::RxBLEBattery,
      InputType::Rssi => Endpoint::Generic0,
      InputType::Button => Endpoint::Generic1,
      InputType::Pressure => Endpoint::Generic2,
      InputType::Unknown | InputType::Depth | InputType::Position => Endpoint::Generic3,
    };

    let msg = HardwareUnsubscribeCmd::new(feature_id, endpoint);
    async move { device.unsubscribe(&msg).await }.boxed()
  }
}
