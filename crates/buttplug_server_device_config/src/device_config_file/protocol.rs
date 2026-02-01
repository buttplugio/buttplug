// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};

use super::device::ConfigBaseDeviceDefinition;

use crate::ProtocolCommunicationSpecifier;

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub(super) struct ProtocolDefinition {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub communication: Option<Vec<ProtocolCommunicationSpecifier>>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub defaults: Option<ConfigBaseDeviceDefinition>,
  #[serde(default)]
  pub configurations: Vec<ConfigBaseDeviceDefinition>,
}
