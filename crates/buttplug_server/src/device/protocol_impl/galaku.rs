// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use async_trait::async_trait;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use uuid::{uuid, Uuid};

use futures_util::future::BoxFuture;
use futures_util::{future, FutureExt};

use buttplug_core::message::{InputData, InputReadingV4, InputType, InputTypeData};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;

use buttplug_server_device_config::{
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};

use crate::device::{
  hardware::{
    Hardware,
    HardwareCommand,
    HardwareEvent,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
  },
  protocol::{
    generic_protocol_initializer_setup,
    ProtocolHandler,
    ProtocolIdentifier,
    ProtocolInitializer,
  },
};

static KEY_TAB: [[u32; 12]; 4] = [
  [0, 24, 152, 247, 165, 61, 13, 41, 37, 80, 68, 70],
  [0, 69, 110, 106, 111, 120, 32, 83, 45, 49, 46, 55],
  [0, 101, 120, 32, 84, 111, 121, 115, 10, 142, 157, 163],
  [0, 197, 214, 231, 248, 10, 50, 32, 111, 98, 13, 10],
];

fn get_tab_key(r: usize, t: usize) -> u32 {
  let e = 3 & r;
  KEY_TAB[e][t]
}

fn encrypt(data: Vec<u32>) -> Vec<u32> {
  let mut new_data = vec![data[0]];
  for i in 1..data.len() {
    let a = get_tab_key(new_data[i - 1] as usize, i);
    let u = (a ^ data[0] ^ data[i]) + a;
    new_data.push(u);
  }
  new_data
}

fn decrypt(data: Vec<u32>) -> Vec<u32> {
  let mut new_data = vec![data[0]];
  for i in 1..data.len() {
    let a = get_tab_key(data[i - 1] as usize, i);
    let u = (data[i] as i32 - a as i32) ^ data[0] as i32 ^ a as i32;
    new_data.push(if u < 0 { (u + 256) as u32 } else { u as u32 });
  }
  new_data
}

fn send_bytes(data: Vec<u32>) -> Vec<u8> {
  let mut new_data = vec![35];
  new_data.extend(data);
  new_data.push(new_data.iter().sum());
  let mut uint8_array: Vec<u8> = Vec::new();
  for value in encrypt(new_data) {
    uint8_array.push(value as u8);
  }
  uint8_array
}

fn read_value(data: Vec<u8>) -> u32 {
  let mut uint32_data: Vec<u32> = Vec::new();
  for value in data {
    uint32_data.push(value as u32);
  }
  let decrypted_data = decrypt(uint32_data);
  if !decrypted_data.is_empty() {
    decrypted_data[4]
  } else {
    0
  }
}

const GALAKU_PROTOCOL_UUID: Uuid = uuid!("766d15d5-0f43-4768-a73a-96ff48bc389e");
generic_protocol_initializer_setup!(Galaku, "galaku");

#[derive(Default)]
pub struct GalakuInitializer {}

#[async_trait]
impl ProtocolInitializer for GalakuInitializer {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    let mut protocol = Galaku::default();
    protocol.is_caiping_pump_device = false;
    if hardware.name() == "AC695X_1(BLE)" {
      protocol.is_caiping_pump_device = true;
    }
    Ok(Arc::new(protocol))
  }
}

pub struct Galaku {
  is_caiping_pump_device: bool,
  speeds: [AtomicU8; 2],
}

impl Default for Galaku {
  fn default() -> Self {
    Self {
      is_caiping_pump_device: false,
      speeds: [AtomicU8::new(0), AtomicU8::new(0)],
    }
  }
}

impl ProtocolHandler for Galaku {
  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if self.is_caiping_pump_device {
      let data: Vec<u8> = vec![
        0xAA,
        1,
        10,
        3,
        speed as u8,
        if speed == 0 { 0 } else { 1 },
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
      ];
      Ok(vec![HardwareWriteCmd::new(
        &[GALAKU_PROTOCOL_UUID],
        Endpoint::Tx,
        data,
        false,
      )
      .into()])
    } else {
      self.speeds[feature_index as usize].store(speed as u8, Ordering::Relaxed);
      let data: Vec<u32> = vec![
        90,
        0,
        0,
        1,
        49,
        self.speeds[0].load(Ordering::Relaxed) as u32,
        self.speeds[1].load(Ordering::Relaxed) as u32,
        0,
        0,
        0,
      ];
      Ok(vec![HardwareWriteCmd::new(
        &[GALAKU_PROTOCOL_UUID],
        Endpoint::Tx,
        send_bytes(data),
        false,
      )
      .into()])
    }
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device_index: u32,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    match sensor_type {
      InputType::Battery => {
        async move {
          device
            .subscribe(&HardwareSubscribeCmd::new(
              feature_id,
              Endpoint::RxBLEBattery,
            ))
            .await?;
          Ok(())
        }
      }
      .boxed(),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this sensor".to_string(),
      )))
      .boxed(),
    }
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    _feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    match sensor_type {
      InputType::Battery => {
        async move {
          device
            .unsubscribe(&HardwareUnsubscribeCmd::new(
              feature_id,
              Endpoint::RxBLEBattery,
            ))
            .await?;
          Ok(())
        }
      }
      .boxed(),
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this sensor".to_string(),
      )))
      .boxed(),
    }
  }

  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    let data: Vec<u32> = vec![90, 0, 0, 1, 19, 0, 0, 0, 0, 0];
    let mut device_notification_receiver = device.event_stream();
    async move {
      device
        .subscribe(&HardwareSubscribeCmd::new(
          feature_id,
          Endpoint::RxBLEBattery,
        ))
        .await?;
      device
        .write_value(&HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          send_bytes(data),
          true,
        ))
        .await?;
      while let Ok(event) = device_notification_receiver.recv().await {
        return match event {
          HardwareEvent::Notification(_, endpoint, data) => {
            if endpoint != Endpoint::RxBLEBattery {
              continue;
            }
            let battery_reading = InputReadingV4::new(
              device_index,
              feature_index,
              InputTypeData::Battery(InputData::new(data[0]))
            );
            Ok(battery_reading)
          }
          HardwareEvent::Disconnected(_) => Err(ButtplugDeviceError::ProtocolSpecificError(
            "Galaku".to_owned(),
            "Galaku Device disconnected while getting Battery info.".to_owned(),
          )),
        };
      }
      Err(ButtplugDeviceError::ProtocolSpecificError(
        "Galaku".to_owned(),
        "Galaku Device disconnected while getting Battery info.".to_owned(),
      ))
    }
    .boxed()
  }
}
