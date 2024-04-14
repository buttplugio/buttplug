use getset::{Getters, Setters, MutGetters, CopyGetters};
use serde::{Serialize, Deserialize};

use crate::core::message::DeviceFeature;

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
  pub fn new(
    name: &str,
    features: &[DeviceFeature],
  ) -> Self {
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

#[derive(Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  /// Message attributes for this device instance.
  features: Vec<DeviceFeature>,
  /// Per-user configurations specific to this device instance.
  user_config: UserDeviceCustomization
}

impl UserDeviceDefinition {
  /// Create a new instance
  pub fn new(
    name: &str,
    features: &[DeviceFeature],
    user_config: &UserDeviceCustomization
  ) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
      user_config: user_config.clone()
    }
  }
}

impl From<BaseDeviceDefinition> for UserDeviceDefinition {
  fn from(value: BaseDeviceDefinition) -> Self {
    Self::new(value.name(), value.features(), &UserDeviceCustomization::default())
  }
}
