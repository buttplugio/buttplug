// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct VectorSubcommand {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Index"))]
    pub index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Duration"))]
    pub duration: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Position"))]
    pub position: f64,
}

impl VectorSubcommand {
    pub fn new(index: u32, duration: u32, position: f64) -> Self {
        Self {
            index,
            duration,
            position,
        }
    }
}

#[derive(Debug, Default, ButtplugDeviceMessage, ButtplugUpgradableMessage, PartialEq, Clone)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct LinearCmd {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "DeviceIndex"))]
    pub device_index: u32,
    #[cfg_attr(feature = "serialize_json", serde(rename = "Vectors"))]
    pub vectors: Vec<VectorSubcommand>,
}

impl LinearCmd {
    pub fn new(device_index: u32, vectors: Vec<VectorSubcommand>) -> Self {
        Self {
            id: 1,
            device_index,
            vectors,
        }
    }
}