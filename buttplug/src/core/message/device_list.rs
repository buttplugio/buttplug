// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::{DeviceMessageInfoV0, DeviceMessageInfoV1, DeviceMessageInfoV2};
use super::*;
use getset::Getters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// List of all devices currently connected to the server.
#[derive(Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceList {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Devices"))]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfo>,
}

impl DeviceList {
  pub fn new(devices: Vec<DeviceMessageInfo>) -> Self {
    Self { id: 1, devices }
  }
}

impl ButtplugMessageValidator for DeviceList {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceList {
  fn finalize(&mut self) {
    for device in &mut self.devices {
      device.device_messages_mut().finalize();
    }
  }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceListV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Devices"))]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfoV2>,
}

impl From<DeviceList> for DeviceListV2 {
  fn from(msg: DeviceList) -> Self {
    let mut devices = vec![];
    for d in msg.devices {
      devices.push(DeviceMessageInfoV2::from(d));
    }
    Self {
      id: msg.id,
      devices,
    }
  }
}

impl ButtplugMessageValidator for DeviceListV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV2 {
}

#[derive(Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceListV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Devices"))]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfoV1>,
}

impl From<DeviceList> for DeviceListV1 {
  fn from(msg: DeviceList) -> Self {
    let mut devices = vec![];
    for d in msg.devices {
      let dmiv2 = DeviceMessageInfoV2::from(d);
      devices.push(DeviceMessageInfoV1::from(dmiv2));
    }
    Self {
      id: msg.id,
      devices,
    }
  }
}

impl ButtplugMessageValidator for DeviceListV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV1 {
}

#[derive(Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceListV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Devices"))]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfoV0>,
}

impl From<DeviceList> for DeviceListV0 {
  fn from(msg: DeviceList) -> Self {
    let mut devices = vec![];
    for d in msg.devices {
      let dmiv2 = DeviceMessageInfoV2::from(d);
      let dmiv1 = DeviceMessageInfoV1::from(dmiv2);
      devices.push(DeviceMessageInfoV0::from(dmiv1));
    }
    Self {
      id: msg.id,
      devices,
    }
  }
}

impl ButtplugMessageValidator for DeviceListV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV0 {
}
