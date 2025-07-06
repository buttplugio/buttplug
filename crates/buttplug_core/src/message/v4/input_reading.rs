// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  InputType,
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, CopyGetters)]
#[getset(get_copy = "pub")]
pub struct InputData<T> where T: Copy + Clone {
  #[serde(rename = "Data")]
  data: T,
}

impl<T> InputData<T> where T: Copy + Clone {
  pub fn new(data: T) -> Self {
    Self { data }
  }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputTypeData {
  Battery(InputData<u8>),
  Rssi(InputData<i8>),
  Button(InputData<u8>),
  Pressure(InputData<u32>)
}

impl InputTypeData {
  pub fn as_input_type(&self) -> InputType {
    match self {
      Self::Battery(_) => InputType::Battery,
      Self::Rssi(_) => InputType::Rssi,
      Self::Button(_) => InputType::Button,
      Self::Pressure(_) => InputType::Pressure,
    }
  }
}

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
pub struct InputReadingV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[serde(rename = "FeatureIndex")]
  #[getset[get_copy="pub"]]
  feature_index: u32,
  #[serde(rename = "Data")]
  #[getset[get_copy="pub"]]
  data: InputTypeData,
}

impl InputReadingV4 {
  pub fn new(
    device_index: u32,
    feature_index: u32,
    data: InputTypeData
  ) -> Self {
    Self {
      id: 0,
      device_index,
      feature_index,
      data,
    }
  }
}
