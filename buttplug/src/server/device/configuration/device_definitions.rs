use getset::{CopyGetters, Getters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::message::server_device_feature::{ServerBaseDeviceFeature, ServerDeviceFeature, ServerUserDeviceFeature};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeviceSettings {
  #[serde(rename = "feature-timing-gap", skip_serializing_if = "Option::is_none", default)]
  feature_timing_gap: Option<u32>,
}

impl DeviceSettings {
  pub fn is_none(&self) -> bool {
    self.feature_timing_gap.is_none()
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, CopyGetters)]
pub struct BaseFeatureSettings {
  #[serde(rename = "alt-protocol-index", skip_serializing_if = "Option::is_none", default)]
  #[getset(get_copy = "pub")]
  alt_protocol_index: Option<u32>,
}

impl BaseFeatureSettings {
  pub fn is_none(&self) -> bool {
    self.alt_protocol_index.is_none()
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserFeatureSettings {
  #[serde(rename = "reverse-position", skip_serializing_if = "Option::is_none", default)]
  reverse_position: Option<bool>
}

impl UserFeatureSettings {
  pub fn is_none(&self) -> bool {
    self.reverse_position.is_none()
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct BaseDeviceDefinition {
  #[getset(get = "pub")]
  /// Given name of the device this instance represents.
  name: String,
  #[getset(get = "pub")]
  /// Message attributes for this device instance.
  features: Vec<ServerBaseDeviceFeature>,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  protocol_variant: Option<String>,
  #[getset(get = "pub")]
  device_settings: DeviceSettings,
}

impl BaseDeviceDefinition {
  /// Create a new instance
  pub fn new(name: &str, id: &Uuid, protocol_variant: &Option<String>, features: &[ServerBaseDeviceFeature], device_settings: &Option<DeviceSettings>) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
      id: *id,
      protocol_variant: protocol_variant.clone(),
      device_settings: device_settings.clone().unwrap_or_default()
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
      index,
    }
  }

  pub fn default_with_index(index: u32) -> Self {
    Self::new(&None, false, false, index)
  }
}

#[derive(Debug, Clone, Getters, Serialize, Deserialize, CopyGetters)]
pub struct UserDeviceDefinition {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  #[serde(rename="base-id")]
  base_id: Uuid,
  #[getset(get = "pub")]
  /// Message attributes for this device instance.
  #[getset(get = "pub")]
  features: Vec<ServerUserDeviceFeature>,
  #[getset(get = "pub")]
  #[serde(rename="user-config")]
  /// Per-user configurations specific to this device instance.
  user_config: UserDeviceCustomization,
}

impl UserDeviceDefinition {
  fn new(index: u32, base_id: Uuid, features: &Vec<ServerUserDeviceFeature>) -> Self {
    Self {
      id: Uuid::new_v4(),
      base_id,
      features: features.clone(),
      user_config: UserDeviceCustomization::default_with_index(index)
    }
  }
}

#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct DeviceDefinition {
  #[getset(get = "pub")]
  base_device: BaseDeviceDefinition,
  #[getset(get = "pub")]
  user_device: UserDeviceDefinition,
  #[getset(get = "pub")]
  features: Vec<ServerDeviceFeature>
}

impl DeviceDefinition {
  /// Create a new instance
  pub fn new(
    base_device: &BaseDeviceDefinition,
    user_device: &UserDeviceDefinition
  ) -> Self {
    let mut features = vec!();
    base_device
      .features()
      .iter()
      .for_each(|x| {
        if let Some(user_feature) = user_device.features.iter().find(|user_feature| user_feature.base_id() == x.id()) {
          features.push(ServerDeviceFeature::new(x, user_feature));
        }
      });
    Self {
      base_device: base_device.clone(),
      user_device: user_device.clone(),
      features
    }
  }

  pub fn id(&self) -> Uuid {
    self.user_device.id()
  }

  pub fn name(&self) -> &str {
    self.base_device.name()
  }

  pub fn protocol_variant(&self) -> &Option<String> {
    self.base_device.protocol_variant()
  }

  pub fn user_config(&self) -> &UserDeviceCustomization {
    self.user_device.user_config()
  }

  pub fn new_from_base_definition(def: &BaseDeviceDefinition, index: u32) -> Self {
    let user_features = def.features().iter().map(|x| x.as_user_device_feature()).collect();
    Self::new(
      def,
      &UserDeviceDefinition::new(index, def.id(), &user_features)
    )
  }
}
