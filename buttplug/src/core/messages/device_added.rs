// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::{DeviceMessageInfoV0, DeviceMessageInfoV1};
use super::*;

#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAdded {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  device_messages: DeviceMessageAttributesMap,
}

impl DeviceAdded {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_messages: &DeviceMessageAttributesMap,
  ) -> Self {
    Self {
      id: 0,
      device_index,
      device_name: device_name.to_string(),
      device_messages: device_messages.clone(),
    }
  }

  pub fn device_index(&self) -> u32 {
    self.device_index
  }

  pub fn device_name(&self) -> &String {
    &self.device_name
  }

  pub fn device_messages(&self) -> &DeviceMessageAttributesMap {
    &self.device_messages
  }
}

impl ButtplugMessageValidator for DeviceAdded {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAddedV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  device_messages: DeviceMessageAttributesMap,
}

impl From<DeviceAdded> for DeviceAddedV1 {
  fn from(msg: DeviceAdded) -> Self {
    let id = msg.id();
    let dmi = DeviceMessageInfo::from(msg);
    let dmiv1 = DeviceMessageInfoV1::from(dmi);

    Self {
      id,
      device_index: dmiv1.device_index,
      device_name: dmiv1.device_name,
      device_messages: dmiv1.device_messages,
    }
  }
}

impl ButtplugMessageValidator for DeviceAddedV1 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAddedV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  device_messages: Vec<ButtplugDeviceMessageType>,
}

impl From<DeviceAdded> for DeviceAddedV0 {
  fn from(msg: DeviceAdded) -> Self {
    let id = msg.id();
    let dmi = DeviceMessageInfo::from(msg);
    let dmiv1 = DeviceMessageInfoV1::from(dmi);
    let dmiv0 = DeviceMessageInfoV0::from(dmiv1);

    Self {
      id,
      device_index: dmiv0.device_index,
      device_name: dmiv0.device_name,
      device_messages: dmiv0.device_messages,
    }
  }
}

impl ButtplugMessageValidator for DeviceAddedV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

// TODO Test repeated message type in attributes in JSON