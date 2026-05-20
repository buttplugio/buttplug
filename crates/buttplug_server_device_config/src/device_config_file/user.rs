// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use dashmap::DashMap;
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::UserDeviceIdentifier;

use super::{
  ConfigVersion,
  ConfigVersionGetter,
  device::ConfigUserDeviceDefinition,
  get_internal_config_version,
  protocol::ProtocolDefinition,
};

#[derive(Deserialize, Serialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct UserDeviceConfigPair {
  pub identifier: UserDeviceIdentifier,
  pub config: ConfigUserDeviceDefinition,
}

#[derive(Deserialize, Serialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub(crate)")]
pub struct SimulatedDeviceConfigEntry {
  pub identifier: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub display_name: Option<String>,
  #[serde(default = "SimulatedDeviceConfigEntry::generate_address")]
  pub address: String,
}

impl SimulatedDeviceConfigEntry {
  pub fn new(identifier: &str, display_name: Option<String>) -> Self {
    Self {
      identifier: identifier.to_owned(),
      display_name,
      address: Self::generate_address(),
    }
  }

  fn generate_address() -> String {
    format!("simulated:{}", Uuid::new_v4())
  }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserConfigDefinition {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub protocols: Option<DashMap<String, ProtocolDefinition>>,
  #[serde(rename = "devices", default, skip_serializing_if = "Option::is_none")]
  pub user_device_configs: Option<Vec<UserDeviceConfigPair>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub simulated_devices: Option<Vec<SimulatedDeviceConfigEntry>>,
}

#[derive(Deserialize, Serialize, Debug, Getters, Setters)]
#[getset(get = "pub", get_mut = "pub", set = "pub")]
pub struct UserConfigFile {
  version: ConfigVersion,
  #[serde(default)]
  user_configs: Option<UserConfigDefinition>,
}

impl Default for UserConfigFile {
  fn default() -> Self {
    Self {
      version: get_internal_config_version(),
      user_configs: Some(UserConfigDefinition::default()),
    }
  }
}

impl ConfigVersionGetter for UserConfigFile {
  fn version(&self) -> ConfigVersion {
    self.version
  }
}

impl UserConfigFile {
  pub fn new(major_version: u32, minor_version: u32) -> Self {
    Self {
      version: ConfigVersion {
        major: major_version,
        minor: minor_version,
      },
      user_configs: None,
    }
  }

  #[allow(dead_code)]
  pub fn to_json(&self) -> String {
    serde_json::to_string(self)
      .expect("All types below this are Serialize, so this should be infallible.")
  }
}
