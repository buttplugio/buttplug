// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugMessageError,
    message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator, DeviceAddedV4},
  },
  server::message::{
    v0::DeviceMessageInfoV0,
    v1::DeviceMessageInfoV1,
    v2::{DeviceAddedV2, DeviceMessageInfoV2},
  },
};

use getset::{CopyGetters, Getters};

#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::{ClientDeviceMessageAttributesV3, DeviceMessageInfoV3};

/// Notification that a device has been found and connected to the server.
#[derive(ButtplugMessage, Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAddedV3 {
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
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  device_messages: ClientDeviceMessageAttributesV3,
}

impl DeviceAddedV3 {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: &Option<u32>,
    device_messages: &ClientDeviceMessageAttributesV3,
  ) -> Self {
    let mut obj = Self {
      id: 0,
      device_index,
      device_name: device_name.to_string(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap: *device_message_timing_gap,
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