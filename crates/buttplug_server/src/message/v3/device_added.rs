// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v0::DeviceMessageInfoV0,
  v1::DeviceMessageInfoV1,
  v2::{DeviceAddedV2, DeviceMessageInfoV2},
};
use buttplug_core::{
  errors::ButtplugMessageError,
  message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceFeature, DeviceMessageInfoV4},
};

use getset::{CopyGetters, Getters};

use serde::{Deserialize, Serialize};

use super::{ClientDeviceMessageAttributesV3, DeviceMessageInfoV3};

/// Notification that a device has been found and connected to the server.
#[derive(
  ButtplugMessage, PartialEq, Clone, Debug, Getters, CopyGetters, Serialize, Deserialize,
)]
pub struct DeviceAddedV3 {
  #[serde(rename = "Id")]
  id: u32,
  // DeviceAdded is not considered a device message because it only notifies of existence and is not
  // a command (and goes from server to client), therefore we have to define the getter ourselves.
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  device_name: String,
  #[serde(rename = "DeviceDisplayName", skip_serializing_if = "Option::is_none")]
  #[getset(get = "pub")]
  device_display_name: Option<String>,
  #[serde(rename = "DeviceMessageTimingGap")]
  #[getset(get_copy = "pub")]
  device_message_timing_gap: u32,
  #[serde(rename = "DeviceMessages")]
  #[getset(get = "pub")]
  device_messages: ClientDeviceMessageAttributesV3,
}

impl DeviceAddedV3 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: u32,
    device_messages: &ClientDeviceMessageAttributesV3,
  ) -> Self {
    let mut obj = Self {
      id: 0,
      device_index,
      device_name: device_name.to_string(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap,
      device_messages: device_messages.clone(),
    };
    obj.finalize();
    obj
  }
}

impl ButtplugMessageValidator for DeviceAddedV3 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV3 {
  fn finalize(&mut self) {
    self.device_messages.finalize();
  }
}

impl From<DeviceAddedV3> for DeviceMessageInfoV0 {
  fn from(device_added: DeviceAddedV3) -> Self {
    let dmi = DeviceMessageInfoV3::from(device_added);
    let dmi_v2: DeviceMessageInfoV2 = dmi.into();
    let dmi_v1: DeviceMessageInfoV1 = dmi_v2.into();
    dmi_v1.into()
  }
}

impl From<DeviceAddedV3> for DeviceAddedV2 {
  fn from(msg: DeviceAddedV3) -> Self {
    let id = msg.id();
    let dmi = DeviceMessageInfoV3::from(msg);
    let dmiv1 = DeviceMessageInfoV2::from(dmi);

    Self {
      id,
      device_index: dmiv1.device_index(),
      device_name: dmiv1.device_name().clone(),
      device_messages: dmiv1.device_messages().clone(),
    }
  }
}

impl From<DeviceAddedV3> for DeviceMessageInfoV2 {
  fn from(device_added: DeviceAddedV3) -> Self {
    let dmi = DeviceMessageInfoV3::from(device_added);
    DeviceMessageInfoV2::from(dmi)
  }
}

impl From<DeviceMessageInfoV4> for DeviceAddedV3 {
  fn from(value: DeviceMessageInfoV4) -> Self {
    let feature_vec: Vec<DeviceFeature> = value.device_features().values().cloned().collect();
    let mut da3 = DeviceAddedV3::new(
      value.device_index(),
      value.device_name(),
      value.device_display_name(),
      value.device_message_timing_gap(),
      &feature_vec.into()
    );
    da3.set_id(0);
    da3
  }
}
