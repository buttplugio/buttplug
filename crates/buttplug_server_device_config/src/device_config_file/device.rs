// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use compact_str::CompactString;
use getset::{CopyGetters, Getters, MutGetters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ButtplugDeviceConfigError, ServerDeviceDefinition, ServerDeviceDefinitionBuilder};

use super::feature::{ConfigBaseDeviceFeature, ConfigUserDeviceFeature};

#[derive(Debug, Clone, Getters, CopyGetters, Serialize, Deserialize)]
pub struct ConfigBaseDeviceDefinition {
  #[getset(get = "pub")]
  pub identifier: Option<Vec<CompactString>>,
  #[getset(get = "pub")]
  /// Given name of the device this instance represents.
  name: CompactString,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get = "pub")]
  protocol_variant: Option<CompactString>,
  #[getset(get_copy = "pub")]
  message_gap_ms: Option<u32>,
  #[getset(get = "pub")]
  features: Option<Vec<ConfigBaseDeviceFeature>>,
}

impl ConfigBaseDeviceDefinition {
  pub fn with_defaults(self, defaults: Option<&ConfigBaseDeviceDefinition>) -> Self {
    Self {
      identifier: self.identifier,
      name: self.name,
      id: self.id,
      protocol_variant: self
        .protocol_variant
        .or_else(|| defaults.and_then(|d| d.protocol_variant.clone())),
      message_gap_ms: self
        .message_gap_ms
        .or_else(|| defaults.and_then(|d| d.message_gap_ms)),
      features: self
        .features
        .or_else(|| defaults.and_then(|d| d.features.clone())),
    }
  }
}

impl From<ConfigBaseDeviceDefinition> for ServerDeviceDefinition {
  fn from(val: ConfigBaseDeviceDefinition) -> Self {
    ServerDeviceDefinitionBuilder::new_with_features(
      val.name,
      val.id,
      val
        .features
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
    )
    .protocol_variant(val.protocol_variant)
    .message_gap_ms(val.message_gap_ms)
    .finish()
  }
}

#[derive(Serialize, Deserialize, Debug, Getters, CopyGetters, Default, Clone, MutGetters)]
pub struct ConfigUserDeviceCustomization {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[getset(get = "pub")]
  display_name: Option<CompactString>,
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
    if self.features().len() != base.features().len() {
      return Err(ButtplugDeviceConfigError::UserFeatureMismatch);
    }

    let mut builder = ServerDeviceDefinitionBuilder::from_base(base, self.id, false)
      .display_name(self.user_config.display_name.clone())
      .message_gap_ms(self.user_config.message_gap_ms)
      .allow(self.user_config.allow)
      .deny(self.user_config.deny)
      .index(self.user_config.index);

    for feature in self.features() {
      if let Some(base_feature) = base
        .features()
        .values()
        .find(|x| x.id() == feature.base_id())
      {
        builder = builder.add_feature(feature.with_base_feature(base_feature)?);
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
