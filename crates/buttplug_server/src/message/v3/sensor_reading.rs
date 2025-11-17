// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_core::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  InputType,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

// This message can have an Id of 0, as it can be emitted as part of a
// subscription and won't have a matching task Id in that case.
#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageValidator,
  ButtplugMessageFinalizer,
  Clone,
  Getters,
  CopyGetters,
  PartialEq,
  Eq,
  Serialize,
  Deserialize,
)]
pub struct SensorReadingV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "SensorIndex")]
  #[getset[get_copy="pub"]]
  sensor_index: u32,
  #[serde(rename = "SensorType")]
  #[getset[get_copy="pub"]]
  sensor_type: InputType,
  #[serde(rename = "Data")]
  #[getset[get="pub"]]
  data: Vec<i32>,
}

impl SensorReadingV3 {
  pub fn new(device_index: u32, sensor_index: u32, sensor_type: InputType, data: Vec<i32>) -> Self {
    Self {
      id: 0,
      device_index,
      sensor_index,
      sensor_type,
      data,
    }
  }
}
