// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{core::{errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError}, message::{
  ButtplugDeviceMessage,
  ButtplugMessage,
  ButtplugMessageFinalizer,
  ButtplugMessageValidator,
  Endpoint, RawReadCmdV2,
}}, server::message::TryFromDeviceAttributes};
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct CheckedRawReadCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "ExpectedLength"))]
  #[getset(get_copy = "pub")]
  expected_length: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Timeout"))]
  #[getset(get_copy = "pub")]
  timeout: u32,
}

impl CheckedRawReadCmdV2 {
  pub fn new(device_index: u32, endpoint: Endpoint, expected_length: u32, timeout: u32) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      expected_length,
      timeout,
    }
  }
}

impl ButtplugMessageValidator for CheckedRawReadCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

impl TryFromDeviceAttributes<RawReadCmdV2> for CheckedRawReadCmdV2 {
  fn try_from_device_attributes(
    msg: RawReadCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    // Find the raw feature.
    if let Some(raw_feature) = features.features().iter().find(|x| x.raw().is_some()) {
      if raw_feature.raw().as_ref().unwrap().endpoints().contains(&msg.endpoint()) {
        Ok(CheckedRawReadCmdV2 { id: msg.id(), device_index: msg.device_index(), endpoint: msg.endpoint(), expected_length: msg.expected_length(), timeout: msg.timeout() })
      } else {
        Err(ButtplugError::from(
          ButtplugDeviceError::InvalidEndpoint(msg.endpoint())
        ))
      }
    } else {
      Err(ButtplugError::from(
        ButtplugDeviceError::DeviceNoRawError("RawReadCmd".to_owned())
      ))
    }
  }
}
