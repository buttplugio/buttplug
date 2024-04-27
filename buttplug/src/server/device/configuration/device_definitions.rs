use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};

use crate::core::message::{DeviceFeature, Endpoint};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct BaseDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  /// Message attributes for this device instance.
  features: Vec<DeviceFeature>,
}

impl BaseDeviceDefinition {
  /// Create a new instance
  pub fn new(name: &str, features: &[DeviceFeature]) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters, Default, Clone)]
pub struct UserDeviceCustomization {
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  #[serde(rename = "display-name")]
  #[getset(get = "pub")]
  display_name: Option<String>,
  #[serde(default)]
  #[getset(get_copy = "pub")]
  allow: bool,
  #[serde(default)]
  #[getset(get_copy = "pub")]
  deny: bool,
  #[getset(get_copy = "pub")]
  index: u32,
}

impl UserDeviceCustomization {
  pub fn new(display_name: &Option<String>, allow: bool, deny: bool, index: u32) -> Self {
    Self {
      display_name: display_name.clone(),
      allow,
      deny,
      index
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  /// Message attributes for this device instance.
  features: Vec<DeviceFeature>,
  /// Per-user configurations specific to this device instance.
  #[serde(rename="user-config")]
  user_config: UserDeviceCustomization,
}

impl UserDeviceDefinition {
  /// Create a new instance
  pub fn new(
    name: &str,
    features: &[DeviceFeature],
    user_config: &UserDeviceCustomization,
  ) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
      user_config: user_config.clone(),
    }
  }

  pub fn new_from_base_definition(def: &BaseDeviceDefinition, index: u32) -> Self {
    Self {
      name: def.name().clone(),
      features: def.features().clone(),
      user_config: UserDeviceCustomization {
        index: index,
        .. Default::default()
      }
    }
  }

  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    self
      .features
      .push(DeviceFeature::new_raw_feature(endpoints));
  }
}
