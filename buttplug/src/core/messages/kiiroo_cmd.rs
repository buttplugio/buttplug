// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::{ButtplugMessage, ButtplugDeviceMessage, ButtplugUpgradableMessage, ButtplugMessageUnion};
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

// Dear god this needs to be deprecated
#[derive(Debug, ButtplugDeviceMessage, ButtplugUpgradableMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct KiirooCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Command"))]
    pub command: String,
}

impl KiirooCmd {
    pub fn new(device_index: u32, command: &str) -> Self {
        Self {
            id: 1,
            device_index,
            command: command.to_owned(),
        }
    }
}
