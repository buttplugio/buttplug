// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
use getset::{CopyGetters, Getters, MutGetters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

/// Substructure of device messages, used for attribute information (name, messages supported, etc...)
#[derive(Clone, Debug, PartialEq, Eq, MutGetters, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfo {
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
  #[getset(get = "pub", get_mut = "pub(super)")]
  device_messages: ClientDeviceMessageAttributes,
}

impl DeviceMessageInfo {
  pub fn new(
    device_index: u32,
    device_name: &str,
    device_display_name: &Option<String>,
    device_message_timing_gap: &Option<u32>,
    device_messages: ClientDeviceMessageAttributes,
  ) -> Self {
    Self {
      device_index,
      device_name: device_name.to_owned(),
      device_display_name: device_display_name.clone(),
      device_message_timing_gap: *device_message_timing_gap,
      device_messages,
    }
  }
}

impl From<DeviceAdded> for DeviceMessageInfo {
  fn from(device_added: DeviceAdded) -> Self {
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_display_name: device_added.device_display_name().clone(),
      device_message_timing_gap: *device_added.device_message_timing_gap(),
      device_messages: device_added.device_messages().clone(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV2 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  device_messages: ClientDeviceMessageAttributesV2,
}

impl From<DeviceAdded> for DeviceMessageInfoV2 {
  fn from(device_added: DeviceAdded) -> Self {
    let dmi = DeviceMessageInfo::from(device_added);
    DeviceMessageInfoV2::from(dmi)
  }
}

impl From<DeviceAddedV2> for DeviceMessageInfoV2 {
  fn from(device_added: DeviceAddedV2) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_messages: device_added.device_messages().clone(),
    }
  }
}

impl From<DeviceMessageInfo> for DeviceMessageInfoV2 {
  fn from(device_message_info: DeviceMessageInfo) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_message_info.device_index,
      device_name: device_message_info.device_name,
      device_messages: device_message_info.device_messages.into(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV1 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  device_messages: ClientDeviceMessageAttributesV1,
}

impl From<DeviceAdded> for DeviceMessageInfoV1 {
  fn from(device_added: DeviceAdded) -> Self {
    let dmi = DeviceMessageInfoV2::from(device_added);
    DeviceMessageInfoV1::from(dmi)
  }
}

impl From<DeviceMessageInfoV2> for DeviceMessageInfoV1 {
  fn from(device_message_info: DeviceMessageInfoV2) -> Self {
    // No structural difference, it's all content changes
    Self {
      device_index: device_message_info.device_index,
      device_name: device_message_info.device_name,
      device_messages: device_message_info.device_messages.into(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct DeviceMessageInfoV0 {
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceIndex"))]
  #[getset(get_copy = "pub")]
  device_index: u32,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceName"))]
  #[getset(get = "pub")]
  device_name: String,
  #[cfg_attr(feature = "serialize-json", serde(rename = "DeviceMessages"))]
  #[getset(get = "pub")]
  device_messages: Vec<ButtplugDeviceMessageType>,
}

impl From<DeviceAdded> for DeviceMessageInfoV0 {
  fn from(device_added: DeviceAdded) -> Self {
    let dmi = DeviceMessageInfo::from(device_added);
    let dmi_v2: DeviceMessageInfoV2 = dmi.into();
    let dmi_v1: DeviceMessageInfoV1 = dmi_v2.into();
    dmi_v1.into()
  }
}

impl From<DeviceMessageInfoV1> for DeviceMessageInfoV0 {
  fn from(device_message_info: DeviceMessageInfoV1) -> Self {
    // Convert to array of message types.
    let mut device_messages: Vec<ButtplugDeviceMessageType> = vec![];

    device_messages.push(ButtplugDeviceMessageType::StopDeviceCmd);
    if device_message_info
      .device_messages
      .single_motor_vibrate_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::SingleMotorVibrateCmd);
    }
    if device_message_info
      .device_messages
      .fleshlight_launch_fw12_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd);
    }
    if device_message_info
      .device_messages
      .vorze_a10_cyclone_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::VorzeA10CycloneCmd);
    }

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
