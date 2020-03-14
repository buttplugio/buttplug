// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2020 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use super::*;
#[cfg(feature = "serialize_json")]
use serde::{Deserialize, Serialize};

#[derive(Debug, ButtplugMessage, ToButtplugMessageUnion, Clone, PartialEq)]
#[cfg_attr(feature = "serialize_json", derive(Serialize, Deserialize))]
pub struct StopScanning {
    #[cfg_attr(feature = "serialize_json", serde(rename = "Id"))]
    pub(super) id: u32,
}

impl Default for StopScanning {
    fn default() -> Self {
        Self { id: 1 }
    }
}
