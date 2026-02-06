// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

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
      features: config.features.or(self.features.clone()),
    }
  }
}

impl From<ConfigBaseDeviceDefinition> for ServerDeviceDefinition {
  fn from(val: ConfigBaseDeviceDefinition) -> Self {
    let mut builder = ServerDeviceDefinitionBuilder::new(&val.name, &val.id);
    if let Some(variant) = val.protocol_variant {
      builder.protocol_variant(&variant);
    }
    if let Some(gap) = val.message_gap_ms {
      builder.message_gap_ms(gap);
    }
    if let Some(features) = val.features {
      for feature in features {
        builder.add_feature(&feature.into());
      }
    }
    builder.finish()
  }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters, Default, Clone, MutGetters)]
pub struct ConfigUserDeviceCustomization {
  #[serde(default, skip_serializing_if = "Option::is_none")]
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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  message_gap_ms: Option<u32>,
}

impl From<&ServerDeviceDefinition> for ConfigUserDeviceCustomization {
  fn from(value: &ServerDeviceDefinition) -> Self {
    Self {
      display_name: value.display_name().clone(),
      allow: value.allow(),
      deny: value.deny(),
      index: value.index(),
      message_gap_ms: value.message_gap_ms(),
    }
  }
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
  pub fn build_from_base_definition(
    &self,
    base: &ServerDeviceDefinition,
  ) -> Result<ServerDeviceDefinition, ButtplugDeviceConfigError> {
    let mut builder = ServerDeviceDefinitionBuilder::from_base(base, self.id, false);
    builder.display_name(&self.user_config.display_name);
    if let Some(message_gap_ms) = self.user_config.message_gap_ms {
      builder.message_gap_ms(message_gap_ms);
    }
    self.user_config.allow.then(|| builder.allow(true));
    self.user_config.deny.then(|| builder.deny(true));
    builder.index(self.user_config.index);
    if self.features().len() != base.features().len() {
      return Err(ButtplugDeviceConfigError::UserFeatureMismatch);
    }
    for feature in self.features() {
      if let Some(base_feature) = base
        .features()
        .values()
        .find(|x| x.id() == feature.base_id())
      {
        builder.add_feature(&feature.with_base_feature(base_feature)?);
      } else {
        return Err(ButtplugDeviceConfigError::UserFeatureMismatch);
      }
    }
    Ok(builder.finish())
  }
}

impl TryFrom<&ServerDeviceDefinition> for ConfigUserDeviceDefinition {
  type Error = ButtplugDeviceConfigError;

  fn try_from(value: &ServerDeviceDefinition) -> Result<Self, Self::Error> {
    Ok(Self {
      id: value.id(),
      base_id: value
        .base_id()
        .ok_or(ButtplugDeviceConfigError::MissingBaseId)?,
      features: value
        .features()
        .values()
        .map(|x| x.try_into())
        .collect::<Result<Vec<_>, _>>()?,
      user_config: value.into(),
    })
  }
}
