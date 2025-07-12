// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::v2::{DeviceListV2, DeviceMessageInfoV2};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceListV4},
};
use getset::Getters;
use serde::{Deserialize, Serialize};

use super::DeviceMessageInfoV3;

/// List of all devices currently connected to the server.
#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage, Getters, Serialize, Deserialize)]
pub struct DeviceListV3 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "Devices")]
  #[getset(get = "pub")]
  devices: Vec<DeviceMessageInfoV3>,
}

impl DeviceListV3 {
  pub fn new(devices: Vec<DeviceMessageInfoV3>) -> Self {
    Self { id: 1, devices }
  }
}

impl ButtplugMessageValidator for DeviceListV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV3 {
  fn finalize(&mut self) {
    for device in &mut self.devices {
      device.device_messages_mut().finalize();
    }
  }
}

impl From<DeviceListV3> for DeviceListV2 {
  fn from(msg: DeviceListV3) -> Self {
    let mut devices = vec![];
    for d in msg.devices() {
      devices.push(DeviceMessageInfoV2::from(d.clone()));
    }
    Self {
      id: msg.id(),
      devices,
    }
  }
}

impl From<DeviceListV4> for DeviceListV3 {
  fn from(value: DeviceListV4) -> Self {
    let mut dl3 = DeviceListV3::new(value.devices().iter().map(|x| x.1.clone().into()).collect());
    dl3.set_id(value.id());
    dl3
  }
}
