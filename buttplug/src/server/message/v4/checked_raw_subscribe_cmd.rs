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
  Endpoint, RawSubscribeCmdV2,
}}, server::message::TryFromDeviceAttributes};
use getset::CopyGetters;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct CheckedRawSubscribeCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
}

impl CheckedRawSubscribeCmdV2 {
  pub fn new(device_index: u32, endpoint: Endpoint) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
    }
  }
}

impl ButtplugMessageValidator for CheckedRawSubscribeCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}

impl TryFromDeviceAttributes<RawSubscribeCmdV2> for CheckedRawSubscribeCmdV2 {
  fn try_from_device_attributes(
    msg: RawSubscribeCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    // Find the raw feature.
    if let Some(raw_feature) = features.features().iter().find(|x| x.raw().is_some()) {
      if raw_feature.raw().as_ref().unwrap().endpoints().contains(&msg.endpoint()) {
        Ok(CheckedRawSubscribeCmdV2 { id: msg.id(), device_index: msg.device_index(), endpoint: msg.endpoint() })
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