// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::{DeviceMessageInfoV0, DeviceMessageInfoV1};
use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceList {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
  pub devices: Vec<DeviceMessageInfo>,
}

impl DeviceList {
  pub fn new(devices: Vec<DeviceMessageInfo>) -> Self {
    Self { id: 1, devices }
  }
}

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceListV1 {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
  pub devices: Vec<DeviceMessageInfoV1>,
}

impl From<DeviceList> for DeviceListV1 {
  fn from(msg: DeviceList) -> Self {
    let mut devices = vec![];
    for d in msg.devices {
      devices.push(DeviceMessageInfoV1::from(d));
    }
    Self {
      id: msg.id,
      devices: devices,
    }
  }
}

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceListV0 {
  #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
  pub(super) id: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
  pub devices: Vec<DeviceMessageInfoV0>,
}

impl From<DeviceList> for DeviceListV0 {
  fn from(msg: DeviceList) -> Self {
    let mut devices = vec![];
    for d in msg.devices {
      let dmiv1 = DeviceMessageInfoV1::from(d);
      devices.push(DeviceMessageInfoV0::from(dmiv1));
    }
    Self {
      id: msg.id,
      devices: devices,
    }
  }
}
