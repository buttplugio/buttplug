// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugMessage,
  ButtplugMessageError,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  DeviceAddedV3,
  DeviceFeature,
};

use getset::{CopyGetters, Getters};

#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Notification that a device has been found and connected to the server.
#[derive(ButtplugMessage, Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAddedV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  // DeviceAdded is not considered a device message because it only notifies of existence and is not
  // a command (and goes from server to client), therefore we have to define the getter ourselves.
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  device_name: String,
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "DeviceDisplayName", skip_serializing_if = "Option::is_none")
  )]
  #[getset(get = "pub")]
  device_display_name: Option<String>,
  #[cfg_attr(
    feature = "serialize-json",
    serde(
      rename = "DeviceMessageTimingGap",
      skip_serializing_if = "Option::is_none"
    )
  )]
  #[getset(get = "pub")]
  device_message_timing_gap: Option<u32>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceFeatures"))]
  #[getset(get = "pub")]
  device_features: Vec<DeviceFeature>,
}

impl DeviceAddedV4 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: &Option<u32>,
    device_features: &Vec<DeviceFeature>,
  ) -> Self {
    let mut obj = Self {
      id: 0,
      device_index,
      device_name: device_name.to_string(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap: *device_message_timing_gap,
      device_features: device_features.clone(),
    };
    obj.finalize();
    obj
  }
}

impl ButtplugMessageValidator for DeviceAddedV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV4 {
  fn finalize(&mut self) {
  }
}

impl From<DeviceAddedV4> for DeviceAddedV3 {
  fn from(value: DeviceAddedV4) -> Self {
    let mut da3 = DeviceAddedV3::new(
      value.device_index(),
      value.device_name(),
      value.device_display_name(),
      &None,
      &value.device_features().clone().into(),
    );
    da3.set_id(value.id());
    da3
  }
}
