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
  Endpoint, RawWriteCmdV2,
}}, server::message::TryFromDeviceAttributes};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

#[derive(
  Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Eq, Clone, Getters, CopyGetters,
)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct CheckedRawWriteCmdV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Endpoint"))]
  #[getset(get_copy = "pub")]
  endpoint: Endpoint,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Data"))]
  #[getset(get = "pub")]
  data: Vec<u8>,
  #[cfg_attr(feature = "serialize-json", serde(rename = "WriteWithResponse"))]
  #[getset(get_copy = "pub")]
  write_with_response: bool,
}

impl CheckedRawWriteCmdV2 {
  pub fn new(
    device_index: u32,
    endpoint: Endpoint,
    data: &[u8],
    write_with_response: bool,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      data: data.to_vec(),
      write_with_response,
    }
  }
}

impl ButtplugMessageValidator for CheckedRawWriteCmdV2 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
  }
}


impl TryFromDeviceAttributes<RawWriteCmdV2> for CheckedRawWriteCmdV2 {
  fn try_from_device_attributes(
    msg: RawWriteCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    // Find the raw feature.
    if let Some(raw_feature) = features.features().iter().find(|x| x.raw().is_some()) {
      if raw_feature.raw().as_ref().unwrap().endpoints().contains(&msg.endpoint()) {
        Ok(CheckedRawWriteCmdV2 { id: msg.id(), device_index: msg.device_index(), endpoint: msg.endpoint(), data: msg.data().clone(), write_with_response: msg.write_with_response() })
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
