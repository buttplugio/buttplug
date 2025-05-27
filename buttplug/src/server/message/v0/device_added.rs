// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::ButtplugMessageError,
    message::{ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator},
  },
};
use getset::{CopyGetters, Getters};

#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

use super::spec_enums::ButtplugDeviceMessageNameV0;

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceAddedV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(in crate::server::message) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  pub(in crate::server::message) device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) device_messages: Vec<ButtplugDeviceMessageNameV0>,
}

impl ButtplugMessageValidator for DeviceAddedV0 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceAddedV0 {
}

// TODO Test repeated message type in attributes in JSON
