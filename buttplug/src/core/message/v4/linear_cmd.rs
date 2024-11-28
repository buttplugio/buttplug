// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  ButtplugDeviceMessage, ButtplugMessage, ButtplugMessageError, ButtplugMessageFinalizer, ButtplugMessageValidator, LegacyDeviceAttributes, LinearCmdV1, TryFromDeviceAttributes
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Move device to a certain position in a certain amount of time
#[derive(Debug, PartialEq, Clone, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
#[getset(get_copy = "pub")]
pub struct VectorSubcommandV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Index"))]
  feature_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Duration"))]
  duration: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Position"))]
  position: f64,
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  id: Option<Uuid>
}

impl VectorSubcommandV4 {
  pub fn new(feature_index: u32, duration: u32, position: f64, id: &Option<Uuid>) -> Self {
    Self {
      feature_index,
      duration,
      position,
      id: id.clone(),
    }
  }
}

#[derive(Debug, ButtplugDeviceMessage, ButtplugMessageFinalizer, PartialEq, Clone, Getters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct LinearCmdV4 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "Id"))]
  id: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "Vectors"))]
  #[getset(get = "pub")]
  vectors: Vec<VectorSubcommandV4>,
}

impl LinearCmdV4 {
  pub fn new(device_index: u32, vectors: Vec<VectorSubcommandV4>) -> Self {
    Self {
      id: 1,
      device_index,
      vectors,
    }
  }
}

impl ButtplugMessageValidator for LinearCmdV4 {
  fn is_valid(&self) -> Result<(), ButtplugMessageError> {
    self.is_not_system_id(self.id)?;
    Ok(())
  }
}

impl TryFromDeviceAttributes<LinearCmdV1> for LinearCmdV4 {
  fn try_from_device_attributes(msg: LinearCmdV1, features: &LegacyDeviceAttributes) -> Result<Self, crate::core::errors::ButtplugError> {
    let cmds: Vec<VectorSubcommandV4> = msg
      .vectors()
      .iter()
      .map(|x| {
        VectorSubcommandV4::new(
          0,
          x.duration(),
          x.position(),
          &Some(features.attrs_v3().linear_cmd().as_ref().unwrap()[x.index() as usize].feature().id().clone())
        )
      })
      .collect();

    Ok(LinearCmdV4::new(msg.device_index(), cmds).into())
  }
}