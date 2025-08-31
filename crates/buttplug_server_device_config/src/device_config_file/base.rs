use std::collections::HashMap;

use getset::Getters;
use serde::{Deserialize, Serialize};

use crate::device_config_file::{get_internal_config_version, protocol::ProtocolDefinition, ConfigVersion, ConfigVersionGetter};


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

impl BaseConfigFile {
  pub(crate) fn new(major_version: u32, minor_version: u32) -> Self {
    Self {
      version: ConfigVersion {
        major: major_version,
        minor: minor_version,
      },
      protocols: None,
    }
  }
}
