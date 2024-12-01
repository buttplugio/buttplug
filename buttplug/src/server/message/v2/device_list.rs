// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::device_message_info::DeviceMessageInfoV2;
use crate::core::{errors::ButtplugMessageError, message::{
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
}};
use getset::Getters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, Eq, ButtplugMessage, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceListV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  pub(in crate::server::message) id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Devices"))]
  #[getset(get = "pub")]
  pub(in crate::server::message) devices: Vec<DeviceMessageInfoV2>,
}

impl ButtplugMessageValidator for DeviceListV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl ButtplugMessageFinalizer for DeviceListV2 {
}
