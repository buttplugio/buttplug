use getset::{CopyGetters, Getters, MutGetters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ServerDeviceDefinition, ServerDeviceDefinitionBuilder};

use super::feature::{ConfigBaseDeviceFeature, ConfigUserDeviceFeature};

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ConfigBaseDeviceDefinition {
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
  features: Vec<ConfigBaseDeviceFeature>,
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
    for feature in self.features {
      builder.add_feature(feature.into());
    }
    builder.finish()
  }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters, Default, Clone, MutGetters)]
pub struct ConfigUserDeviceCustomization {
  #[serde(
    rename = "display-name",
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
    rename = "message-gap-ms",
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
  #[serde(rename = "base-id")]
  base_id: Uuid,
  #[getset(get = "pub")]
  /// Message attributes for this device instance.
  #[getset(get = "pub", get_mut = "pub")]
  features: Vec<ConfigUserDeviceFeature>,
  #[getset(get = "pub", get_mut = "pub")]
  #[serde(rename = "user-config")]
  /// Per-user configurations specific to this device instance.
  user_config: ConfigUserDeviceCustomization,
}
