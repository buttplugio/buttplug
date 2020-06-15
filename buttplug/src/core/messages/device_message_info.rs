// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type MessageAttributesMap = HashMap<ButtplugDeviceMessageType, MessageAttributes>;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfo {
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
  pub device_messages: MessageAttributesMap,
}

impl From<&DeviceAdded> for DeviceMessageInfo {
  fn from(device_added: &DeviceAdded) -> Self {
    Self {
      device_index: device_added.device_index,
      device_name: device_added.device_name.clone(),
      device_messages: device_added.device_messages.clone(),
    }
  }
}

impl From<DeviceAdded> for DeviceMessageInfo {
  fn from(device_added: DeviceAdded) -> Self {
    Self {
      device_index: device_added.device_index,
      device_name: device_added.device_name,
      device_messages: device_added.device_messages,
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV1 {
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
  pub device_messages: MessageAttributesMap,
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
      device_messages: device_message_info.device_messages,
    };
    // Remove entries that weren't in V1.
    let v2_message_types = [
      ButtplugDeviceMessageType::RawReadCmd,
      ButtplugDeviceMessageType::RawWriteCmd,
      ButtplugDeviceMessageType::RawSubscribeCmd,
      ButtplugDeviceMessageType::RawUnsubscribeCmd,
      ButtplugDeviceMessageType::BatteryLevelCmd,
      ButtplugDeviceMessageType::RSSILevelCmd,
      // PatternCmd
      // BatteryLevelReading
      // RSSILevelReading
      // ShockCmd?
      // ToneEmitterCmd?
    ];
    for t in &v2_message_types {
      dmi_v1.device_messages.remove(t);
    }

    // The only attribute in v1 was feature count, so that's all we should
    // preserve.
    for mut attributes in &mut dmi_v1.device_messages.values_mut() {
      let fc = attributes.feature_count;
      *attributes = MessageAttributes::default();
      attributes.feature_count = fc;
    }

    // If VibrateCmd is listed, append SingleMotorVibrateCmd
    if dmi_v1
      .device_messages
      .contains_key(&ButtplugDeviceMessageType::VibrateCmd)
    {
      dmi_v1.device_messages.insert(
        ButtplugDeviceMessageType::SingleMotorVibrateCmd,
        MessageAttributes::default(),
      );
    }

    dmi_v1
  }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV0 {
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
  pub device_index: u32,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
  pub device_name: String,
  #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
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

    // SingleMotorVibrateCmd is added as part of the V1 conversion, so we
    // can expect we'll have it here.
    Self {
      device_name: device_message_info.device_name,
      device_index: device_message_info.device_index,
      device_messages,
    }
  }
}
