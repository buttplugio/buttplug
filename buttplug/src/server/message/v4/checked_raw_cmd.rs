// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError, ButtplugMessageError},
    message::{
      ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageFinalizer, ButtplugMessageValidator,
      Endpoint, RawCommandData, RawCommandRead, RawCommandType, RawCommandWrite,
    },
  },
  server::message::{
    server_device_attributes::ServerDeviceAttributes, RawCmdV2, RawReadCmdV2, RawSubscribeCmdV2, RawUnsubscribeCmdV2, RawWriteCmdV2, TryFromDeviceAttributes
  },
};
use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

#[derive(
  Debug,
  ButtplugDeviceMessage,
  ButtplugMessageFinalizer,
  PartialEq,
  Eq,
  Clone,
  Getters,
  CopyGetters,
  Serialize,
  Deserialize,
)]
pub struct CheckedRawCmdV4 {
  #[serde(rename = "Id")]
  id: u32,
  #[serde(rename = "DeviceIndex")]
  device_index: u32,
  #[getset(get = "pub")]
  #[serde(rename = "Endpoint")]
  endpoint: Endpoint,
  #[getset(get = "pub")]
  #[serde(rename = "RawCommandType")]
  raw_command_type: RawCommandType,
  #[getset(get = "pub")]
  #[serde(rename = "RawCommandData", skip_serializing_if = "Option::is_none")]
  raw_command_data: Option<RawCommandData>,
}

impl CheckedRawCmdV4 {
  pub fn new(
    device_index: u32,
    endpoint: Endpoint,
    raw_command_type: RawCommandType,
    raw_command_data: &Option<RawCommandData>,
  ) -> Self {
    Self {
      id: 1,
      device_index,
      endpoint,
      raw_command_type,
      raw_command_data: raw_command_data.clone(),
    }
  }
}

impl ButtplugMessageValidator for CheckedRawCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)
    // TODO Should expected_length always be > 0?
  }
}

fn check_raw_endpoint<T>(
  msg: &T,
  features: &crate::server::message::ServerDeviceAttributes,
  raw_command_type: RawCommandType,
) -> Result<(), ButtplugError> where T: RawCmdV2 {
  // Find the raw feature.
  if let Some(raw_feature) = features.features().iter().find(|x| x.raw().is_some()) {
    if raw_feature
      .raw()
      .as_ref()
      .unwrap()
      .endpoints()
      .contains(&msg.endpoint())
    {
      Ok(())
    } else {
      Err(ButtplugError::from(ButtplugDeviceError::InvalidEndpoint(
        msg.endpoint(),
      )))
    }
  } else {
    Err(ButtplugError::from(ButtplugDeviceError::DeviceNoRawError(
      format!("{}", raw_command_type),
    )))
  }
}

impl TryFromDeviceAttributes<RawReadCmdV2> for CheckedRawCmdV4 {
  fn try_from_device_attributes(
    msg: RawReadCmdV2,
    features: &ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    check_raw_endpoint(&msg, features, RawCommandType::Read)?;
    Ok(CheckedRawCmdV4 {
      id: msg.id(),
      device_index: msg.device_index(),
      endpoint: msg.endpoint(),
      raw_command_type: RawCommandType::Read,
      raw_command_data: Some(RawCommandData::Read(RawCommandRead::new(
        msg.expected_length(),
        msg.timeout(),
      ))),
    })
  }
}

impl TryFromDeviceAttributes<RawSubscribeCmdV2> for CheckedRawCmdV4 {
  fn try_from_device_attributes(
    msg: RawSubscribeCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    check_raw_endpoint(&msg, features, RawCommandType::Subscribe)?;
    Ok(CheckedRawCmdV4 {
      id: msg.id(),
      device_index: msg.device_index(),
      endpoint: msg.endpoint(),
      raw_command_type: RawCommandType::Subscribe,
      raw_command_data: None,
    })
  }
}

impl TryFromDeviceAttributes<RawUnsubscribeCmdV2> for CheckedRawCmdV4 {
  fn try_from_device_attributes(
    msg: RawUnsubscribeCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    check_raw_endpoint(&msg, features, RawCommandType::Unsubscribe)?;
    Ok(CheckedRawCmdV4 {
      id: msg.id(),
      device_index: msg.device_index(),
      endpoint: msg.endpoint(),
      raw_command_type: RawCommandType::Unsubscribe,
      raw_command_data: None,
    })
  }
}

impl TryFromDeviceAttributes<RawWriteCmdV2> for CheckedRawCmdV4 {
  fn try_from_device_attributes(
    msg: RawWriteCmdV2,
    features: &crate::server::message::ServerDeviceAttributes,
  ) -> Result<Self, crate::core::errors::ButtplugError> {
    check_raw_endpoint(&msg, features, RawCommandType::Write)?;
    Ok(CheckedRawCmdV4 {
      id: msg.id(),
      device_index: msg.device_index(),
      endpoint: msg.endpoint(),
      raw_command_type: RawCommandType::Write,
      raw_command_data: Some(RawCommandData::Write(RawCommandWrite::new(
        msg.data(),
        msg.write_with_response(),
      ))),
    })
  }
}
