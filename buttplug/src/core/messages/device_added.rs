// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugMessage, MessageAttributesMap, ButtplugUpgradableMessage, ButtplugMessageUnion};
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Default, ButtplugMessage, ButtplugUpgradableMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceAdded {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
    pub device_name: String,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
    pub device_messages: MessageAttributesMap,
}

impl DeviceAdded {
    pub fn new(
        device_index: u32,
        device_name: &String,
        device_messages: &MessageAttributesMap,
    ) -> Self {
        Self {
            id: 0,
            device_index,
            device_name: device_name.to_string(),
            device_messages: device_messages.clone(),
        }
    }
}

#[derive(Default, ButtplugMessage, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceAddedVersion0 {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceName"))]
    pub device_name: String,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceMessages"))]
    pub device_messages: Vec<String>,
}

impl From<DeviceAdded> for DeviceAddedVersion0 {
    fn from(msg: DeviceAdded) -> Self {
        Self {
            id: msg.get_id(),
            device_index: msg.device_index,
            device_name: msg.device_name,
            device_messages: msg.device_messages.keys().cloned().collect()
        }
    }
}