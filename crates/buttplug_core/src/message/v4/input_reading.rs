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
pub struct InputValue<T>
where
  T: Copy + Clone,
{
  #[serde(rename = "Value")]
  data: T,
}

impl<T> InputValue<T>
where
  T: Copy + Clone,
{
  pub fn new(data: T) -> Self {
    Self { data }
  }
}

impl From<u8> for InputValue<u8> {
  fn from(value: u8) -> Self {
    InputValue::new(value)
  }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputTypeReading {
  Battery(InputValue<u8>),
  Rssi(InputValue<i8>),
  Button(InputValue<u8>),
  Pressure(InputValue<u32>),
}

impl Into<InputType> for InputTypeReading {
  fn into(self) -> InputType {
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
  #[serde(rename = "Reading")]
  #[getset[get_copy="pub"]]
  reading: InputTypeReading,
}

impl InputReadingV4 {
  pub fn new(device_index: u32, feature_index: u32, data: InputTypeReading) -> Self {
    Self {
      id: 0,
      device_index,
      feature_index,
      reading: data,
    }
  }
}
