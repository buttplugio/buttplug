// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugMessage, ButtplugDeviceMessage, ButtplugUpgradableMessage, ButtplugMessageUnion};
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugDeviceMessage, ButtplugUpgradableMessage, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VorzeA10CycloneCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Speed"))]
    pub speed: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Clockwise"))]
    pub clockwise: bool,
}

impl VorzeA10CycloneCmd {
    pub fn new(device_index: u32, speed: u32, clockwise: bool) -> Self {
        Self {
            id: 1,
            device_index,
            speed,
            clockwise,
        }
    }
}