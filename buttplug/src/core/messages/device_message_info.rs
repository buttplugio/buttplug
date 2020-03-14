// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::*;

pub type MessageAttributesMap = HashMap<String, MessageAttributes>;

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