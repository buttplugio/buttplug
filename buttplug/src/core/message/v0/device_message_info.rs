// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::message::{
  v1::DeviceMessageInfoV1,
  v2::DeviceMessageInfoV2,
  v3::{DeviceAddedV3, DeviceMessageInfoV3},
  ButtplugDeviceMessageType,
};
use getset::{CopyGetters, Getters};
#[cfg(feature = "serialize-json")]
use serde::{Deserialize, Serialize};

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

impl From<DeviceAddedV3> for DeviceMessageInfoV0 {
  fn from(device_added: DeviceAddedV3) -> Self {
    let dmi = DeviceMessageInfoV3::from(device_added);
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
      .device_messages()
      .single_motor_vibrate_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::SingleMotorVibrateCmd);
    }
    if device_message_info
      .device_messages()
      .fleshlight_launch_fw12_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd);
    }
    if device_message_info
      .device_messages()
      .vorze_a10_cyclone_cmd()
      .is_some()
    {
      device_messages.push(ButtplugDeviceMessageType::VorzeA10CycloneCmd);
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
