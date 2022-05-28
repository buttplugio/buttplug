// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};

/// A map pairing device message types (device commands like VibrateCmd, LinearCmd, StopDeviceCmd,
/// etc...) to configuration information about those commands. This includes information about
/// number of features (vibration motor count, rotator count, etc...), power levels and ranges,
/// etc...
///
/// If a message type is in this map, it is assumed to be supported by a device and its protocol.
pub type DeviceMessageAttributesMap = HashMap<ButtplugDeviceMessageType, DeviceMessageAttributes>;

fn ordered_map<S>(value: &DeviceMessageAttributesMap, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let ordered: BTreeMap<_, _> = value.iter().collect();
  ordered.serialize(serializer)
}

/// Substructure of device messages, used for attribute information (name, messages supported, etc...)
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfo {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "DeviceMessages", serialize_with = "ordered_map")
  )]
  pub device_messages: DeviceMessageAttributesMap,
  // We need to store off the original device messages we had passed in, as we
  // may need to include message attributes in earlier versions that are
  // deprecated in later versions.
  #[cfg_attr(feature = "serialize-json", serde(skip))]
  pub original_device_messages: DeviceMessageAttributesMap,
}

impl DeviceMessageInfo {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_messages: DeviceMessageAttributesMap,
  ) -> Self {
    Self {
      device_index,
      device_name: device_name.to_owned(),
      device_messages: device_messages.to_owned(),
      original_device_messages: device_messages,
    }
  }
}

impl From<DeviceAdded> for DeviceMessageInfo {
  fn from(device_added: DeviceAdded) -> Self {
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_messages: device_added.device_messages().clone(),
      original_device_messages: device_added.device_messages().clone(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(
    feature = "serialize-json",
    serde(rename = "DeviceMessages", serialize_with = "ordered_map")
  )]
  pub device_messages: DeviceMessageAttributesMap,
}

impl From<DeviceAdded> for DeviceMessageInfoV1 {
  fn from(device_added: DeviceAdded) -> Self {
    let dmi = DeviceMessageInfo::from(device_added);
    DeviceMessageInfoV1::from(dmi)
  }
}

impl From<DeviceMessageInfo> for DeviceMessageInfoV1 {
  fn from(device_message_info: DeviceMessageInfo) -> Self {
    // No structural difference, it's all content changes
    let mut dmi_v1 = Self {
      device_index: device_message_info.device_index,
      device_name: device_message_info.device_name,
      device_messages: device_message_info.original_device_messages,
    };
    // Remove entries that weren't in V1.
    let v2_message_types = [
      ButtplugDeviceMessageType::RawReadCmd,
      ButtplugDeviceMessageType::RawWriteCmd,
      ButtplugDeviceMessageType::RawSubscribeCmd,
      ButtplugDeviceMessageType::RawUnsubscribeCmd,
      ButtplugDeviceMessageType::BatteryLevelCmd,
      ButtplugDeviceMessageType::RSSILevelCmd,
    ];
    for t in &v2_message_types {
      dmi_v1.device_messages.remove(t);
    }

    // The only attribute in v1 was feature count, so that's all we should
    // preserve.
    for attributes in &mut dmi_v1.device_messages.values_mut() {
      if let Some(feature_count) = attributes.feature_count() {
        *attributes = DeviceMessageAttributesBuilder::default()
          .feature_count(*feature_count)
          .build_without_check();
      }
    }

    // If VibrateCmd is listed, append SingleMotorVibrateCmd
    if dmi_v1
      .device_messages
      .contains_key(&ButtplugDeviceMessageType::VibrateCmd)
    {
      dmi_v1.device_messages.insert(
        ButtplugDeviceMessageType::SingleMotorVibrateCmd,
        DeviceMessageAttributes::default(),
      );
    }

    dmi_v1
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  pub device_messages: Vec<ButtplugDeviceMessageType>,
}

impl From<DeviceAdded> for DeviceMessageInfoV0 {
  fn from(device_added: DeviceAdded) -> Self {
    let dmi = DeviceMessageInfo::from(device_added);
    let dmi_v1: DeviceMessageInfoV1 = dmi.into();
    dmi_v1.into()
  }
}

impl From<DeviceMessageInfoV1> for DeviceMessageInfoV0 {
  fn from(device_message_info: DeviceMessageInfoV1) -> Self {
    // Convert to array of message types.
    let mut device_messages: Vec<ButtplugDeviceMessageType> = device_message_info
      .device_messages
      .keys()
      .cloned()
      .collect();
    // Remove V1 entries that weren't in V0.
    let v1_message_types = [
      ButtplugDeviceMessageType::VibrateCmd,
      ButtplugDeviceMessageType::RotateCmd,
      ButtplugDeviceMessageType::LinearCmd,
    ];

    device_messages.retain(|x| !v1_message_types.contains(x));
    device_messages.sort();

    // SingleMotorVibrateCmd is added as part of the V1 conversion, so we
    // can expect we'll have it here.
    Self {
      device_name: device_message_info.device_name,
      device_index: device_message_info.device_index,
      device_messages,
    }
  }
}
