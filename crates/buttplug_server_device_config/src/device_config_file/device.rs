use getset::{CopyGetters, Getters, MutGetters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ButtplugDeviceConfigError, ServerDeviceDefinition, ServerDeviceDefinitionBuilder};

use super::feature::{ConfigBaseDeviceFeature, ConfigUserDeviceFeature};

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ConfigBaseDeviceDefinition {
  #[getset(get = "pub")]
  identifier: Option<Vec<String>>,
  #[getset(get = "pub")]
  /// Given name of the device this instance represents.
  name: String,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  protocol_variant: Option<String>,
  #[getset(get_copy = "pub")]
  message_gap_ms: Option<u32>,
  #[getset(get = "pub")]
  features: Option<Vec<ConfigBaseDeviceFeature>>,
}

impl ConfigBaseDeviceDefinition {
  pub fn update_with_configuration(&self, config: ConfigBaseDeviceDefinition) -> Self {
    Self {
      identifier: config.identifier().clone(),
      name: config.name().clone(),
      id: config.id(),
      protocol_variant: config.protocol_variant.or(self.protocol_variant.clone()),
      message_gap_ms: config.message_gap_ms.or(self.message_gap_ms),
      features: config.features.or(self.features.clone())
    }
  }
}

impl Into<ServerDeviceDefinition> for ConfigBaseDeviceDefinition {
  fn into(self) -> ServerDeviceDefinition {
    let mut builder = ServerDeviceDefinitionBuilder::new(&self.name, &self.id);
    if let Some(variant) = self.protocol_variant {
      builder.protocol_variant(&variant);
    }
    if let Some(gap) = self.message_gap_ms {
      builder.message_gap_ms(gap);
    }
    if let Some(features) = self.features {
      for feature in features {
        builder.add_feature(&feature.into());
      }
    }
    builder.finish()
  }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters, Default, Clone, MutGetters)]
pub struct ConfigUserDeviceCustomization {
  #[serde(
    default,
    skip_serializing_if = "Option::is_none"
  )]
  #[getset(get = "pub")]
  display_name: Option<String>,
  #[serde(default)]
  #[getset(get_copy = "pub")]
  allow: bool,
  #[serde(default)]
  #[getset(get_copy = "pub")]
  deny: bool,
  #[getset(get_copy = "pub", get_mut = "pub")]
  index: u32,
  #[getset(get_copy = "pub")]
  #[serde(
    default,
    skip_serializing_if = "Option::is_none"
  )]
  message_gap_ms: Option<u32>,
}

#[derive(Debug, Clone, Getters, MutGetters, Serialize, Deserialize, CopyGetters)]
pub struct ConfigUserDeviceDefinition {
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Uuid,
  #[getset(get = "pub")]
  /// Message attributes for this device instance.
  #[getset(get = "pub", get_mut = "pub")]
  features: Vec<ConfigUserDeviceFeature>,
  #[getset(get = "pub", get_mut = "pub")]
  /// Per-user configurations specific to this device instance.
  user_config: ConfigUserDeviceCustomization,
}

impl ConfigUserDeviceDefinition {
  pub fn build_from_base_definition(&self, base: &ServerDeviceDefinition) -> Result<ServerDeviceDefinition, ButtplugDeviceConfigError> {
    let mut builder = ServerDeviceDefinitionBuilder::from_base(&base, self.id);
    if let Some(display_name) = &self.user_config.display_name {
      builder.display_name(display_name);
    }
    if let Some(message_gap_ms) = self.user_config.message_gap_ms {
      builder.message_gap_ms(message_gap_ms);
    }
    self.user_config.allow.then(|| builder.allow());
    self.user_config.deny.then(|| builder.deny());
    builder.index(self.user_config.index);
    if self.features().len() != base.features().len() {
      return Err(ButtplugDeviceConfigError::UserFeatureMismatch);
    }
    for feature in self.features() {
      if let Some(base_feature) = base.features().iter().find(|x| x.id() == feature.base_id()) {
        builder.add_feature(&feature.with_base_feature(base_feature)?);
      } else {
        return Err(ButtplugDeviceConfigError::UserFeatureMismatch);
      }
    }
    Ok(builder.finish())
  }
}