// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceList {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
    pub devices: Vec<DeviceMessageInfo>,
}

impl DeviceList {
    pub fn new(devices: Vec<DeviceMessageInfo>) -> Self {
        Self { id: 0, devices }
    }
}

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceListV1 {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
    pub devices: Vec<DeviceMessageInfo>,
}

impl From<DeviceList> for DeviceListV1 {
    fn from(msg: DeviceList) -> Self {
        Self {
            id: msg.id,
            devices: msg.devices
        }
    }
}

#[derive(Default, Clone, Debug, PartialEq, ButtplugMessage)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct DeviceListV0 {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Devices"))]
    pub devices: Vec<DeviceMessageInfo>,
}

impl From<DeviceList> for DeviceListV0 {
    fn from(msg: DeviceList) -> Self {
        Self {
            id: msg.id,
            devices: msg.devices
        }
    }
}