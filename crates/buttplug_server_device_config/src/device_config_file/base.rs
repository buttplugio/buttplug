// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::collections::HashMap;

use getset::Getters;
use serde::{Deserialize, Serialize};

use crate::device_config_file::{
  ConfigVersion,
  ConfigVersionGetter,
  get_internal_config_version,
  protocol::ProtocolDefinition,
};

#[derive(Deserialize, Serialize, Debug, Getters)]
#[getset(get_mut = "pub", set = "pub")]
pub struct BaseConfigFile {
  #[getset(get_copy = "pub")]
  version: ConfigVersion,
  #[getset(get = "pub")]
  #[serde(default, skip_serializing_if = "Option::is_none")]
  protocols: Option<HashMap<String, ProtocolDefinition>>,
}

impl Default for BaseConfigFile {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      protocols: Some(HashMap::new()),
    }
  }
}

impl ConfigVersionGetter for BaseConfigFile {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}
