// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};

use crate::message::{v0::DeviceMessageInfoV0, ButtplugDeviceMessageNameV0};

use super::{ClientDeviceMessageAttributesV1, DeviceAddedV1};

#[derive(Clone, Debug, PartialEq, Eq, Getters, CopyGetters, Serialize, Deserialize)]
pub struct DeviceMessageInfoV1 {
  #[serde(rename = "DeviceIndex")]
  #[getset(get_copy = "pub")]
  pub(in crate::message) device_index: u32,
  #[serde(rename = "DeviceName")]
  #[getset(get = "pub")]
  pub(in crate::message) device_name: String,
  #[serde(rename = "DeviceMessages")]
  #[getset(get = "pub")]
  pub(in crate::message) device_messages: ClientDeviceMessageAttributesV1,
}

impl From<DeviceAddedV1> for DeviceMessageInfoV1 {
  fn from(device_added: DeviceAddedV1) -> Self {
    Self {
      device_index: device_added.device_index(),
      device_name: device_added.device_name().clone(),
      device_messages: device_added.device_messages().clone(),
    }
  }
}

impl From<DeviceMessageInfoV1> for DeviceMessageInfoV0 {
  fn from(device_message_info: DeviceMessageInfoV1) -> Self {
    // Convert to array of message types.
    let mut device_messages: Vec<ButtplugDeviceMessageNameV0> = vec![];

    device_messages.push(ButtplugDeviceMessageNameV0::StopDeviceCmd);
    if device_message_info
      .device_messages()
      .single_motor_vibrate_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageNameV0::SingleMotorVibrateCmd);
    }
    if device_message_info
      .device_messages()
      .fleshlight_launch_fw12_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageNameV0::FleshlightLaunchFW12Cmd);
    }
    if device_message_info
      .device_messages()
      .vorze_a10_cyclone_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageNameV0::VorzeA10CycloneCmd);
    }

    device_messages.sort();

    // SingleMotorVibrateCmd is added as part of the V1 conversion, so we
    // can expect we'll have it here.
    Self {
      device_name: device_message_info.device_name().clone(),
      device_index: device_message_info.device_index(),
      device_messages,
    }
  }
}
