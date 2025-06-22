use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{core::message::Endpoint, server::message::server_device_feature::ServerDeviceFeature};

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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BaseFeatureSettings {
  #[serde(rename = "alt-protocol-index", skip_serializing_if = "Option::is_none", default)]
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

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct BaseDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  /// Message attributes for this device instance.
  features: Vec<ServerDeviceFeature>,
  id: Uuid,
  protocol_variant: Option<String>,
  device_settings: DeviceSettings,
}

impl BaseDeviceDefinition {
  /// Create a new instance
  pub fn new(name: &str, id: &Uuid, protocol_variant: &Option<String>, features: &[ServerDeviceFeature], device_settings: &Option<DeviceSettings>) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
      id: *id,
      protocol_variant: protocol_variant.clone(),
      device_settings: device_settings.clone().unwrap_or_default()
    }
  }

  pub fn create_user_device_features(&self) -> Vec<ServerDeviceFeature> {
    self
      .features
      .iter()
      .map(|feature| feature.as_user_feature())
      .collect()
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  id: Uuid,
  #[serde(skip_serializing_if = "Option::is_none", rename = "base-id")]
  base_id: Option<Uuid>,
  #[serde(skip_serializing_if = "Option::is_none", rename = "protocol-variant")]
  protocol_variant: Option<String>,
  /// Message attributes for this device instance.
  features: Vec<ServerDeviceFeature>,
  /// Per-user configurations specific to this device instance.
  #[serde(rename = "user-config")]
  user_config: UserDeviceCustomization,
}

impl UserDeviceDefinition {
  /// Create a new instance
  pub fn new(
    name: &str,
    id: &Uuid,
    base_id: &Option<Uuid>,
    protocol_variant: &Option<String>,
    features: &[ServerDeviceFeature],
    user_config: &UserDeviceCustomization,
  ) -> Self {
    Self {
      name: name.to_owned(),
      id: id.to_owned(),
      base_id: base_id.to_owned(),
      protocol_variant: protocol_variant.clone(),
      features: features.into(),
      user_config: user_config.clone(),
    }
  }

  pub fn new_from_base_definition(def: &BaseDeviceDefinition, index: u32) -> Self {
    Self {
      name: def.name().clone(),
      id: Uuid::new_v4(),
      base_id: Some(*def.id()),
      protocol_variant: def.protocol_variant().clone(),
      features: def.create_user_device_features(),
      user_config: UserDeviceCustomization {
        index,
        ..Default::default()
      },
    }
  }

  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    self
      .features
      .push(ServerDeviceFeature::new_raw_feature(endpoints));
  }
}
