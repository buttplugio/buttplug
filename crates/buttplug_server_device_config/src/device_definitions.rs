// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use compact_str::CompactString;
use getset::{CopyGetters, Getters};
use litemap::LiteMap;
use uuid::Uuid;

use super::server_device_feature::ServerDeviceFeature;
#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceDefinition {
  #[getset(get = "pub")]
  /// Given name of the device this instance represents.
  name: CompactString,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Option<Uuid>,
  #[getset(get = "pub")]
  protocol_variant: Option<CompactString>,
  #[getset(get_copy = "pub")]
  message_gap_ms: Option<u32>,
  #[getset(get = "pub")]
  display_name: Option<CompactString>,
  #[getset(get_copy = "pub")]
  allow: bool,
  #[getset(get_copy = "pub")]
  deny: bool,
  #[getset(get_copy = "pub")]
  index: u32,
  // FEATURES MUST BE A BTREEMAP
  //
  // Older versions of the protocol expect specific ordering, so we need to make sure storage
  // adheres to that since we do a lot of value iteration elsewhere.
  #[getset(get = "pub")]
  features: LiteMap<u32, ServerDeviceFeature>,
}

#[derive(Debug)]
pub struct ServerDeviceDefinitionBuilder {
  def: ServerDeviceDefinition,
}

impl ServerDeviceDefinitionBuilder {
  pub fn new(name: CompactString, id: Uuid) -> Self {
    Self {
      def: ServerDeviceDefinition {
        name,
        id,
        base_id: None,
        protocol_variant: None,
        message_gap_ms: None,
        display_name: None,
        allow: false,
        deny: false,
        index: 0,
        features: LiteMap::new(),
      },
    }
  }

  pub fn new_with_features(
    name: CompactString,
    id: Uuid,
    features_iter: impl Iterator<Item = ServerDeviceFeature>,
  ) -> Self {
    // LiteMap's .collect() doesn't take capacity into account
    let mut features = LiteMap::with_capacity(features_iter.size_hint().0);
    for feature in features_iter {
      features.insert(feature.index(), feature);
    }
    Self {
      def: ServerDeviceDefinition {
        name,
        id,
        base_id: None,
        protocol_variant: None,
        message_gap_ms: None,
        display_name: None,
        allow: false,
        deny: false,
        index: 0,
        features,
      },
    }
  }

  // Used to create new user definitions from a base definition.
  pub fn from_base(value: &ServerDeviceDefinition, id: Uuid, with_features: bool) -> Self {
    let mut value = value.clone();
    value.base_id = Some(value.id);
    value.id = id;
    if with_features {
      value.features = value
        .features()
        .iter()
        .map(|(index, x)| {
          let feat = x.as_new_user_feature();
          (*index, feat)
        })
        .collect();
    } else {
      value.features = LiteMap::new();
    }
    ServerDeviceDefinitionBuilder { def: value }
  }

  pub fn from_user(value: &ServerDeviceDefinition) -> Self {
    ServerDeviceDefinitionBuilder { def: value.clone() }
  }

  pub fn id(mut self, id: Uuid) -> Self {
    self.def.id = id;
    self
  }

  pub fn base_id(mut self, id: Uuid) -> Self {
    self.def.base_id = Some(id);
    self
  }

  pub fn display_name(mut self, name: Option<CompactString>) -> Self {
    self.def.display_name = name.clone();
    self
  }

  pub fn protocol_variant(mut self, variant: Option<CompactString>) -> Self {
    self.def.protocol_variant = variant;
    self
  }

  pub fn message_gap_ms(mut self, gap: Option<u32>) -> Self {
    self.def.message_gap_ms = gap;
    self
  }

  pub fn allow(mut self, allow: bool) -> Self {
    self.def.allow = allow;
    self
  }

  pub fn deny(mut self, deny: bool) -> Self {
    self.def.deny = deny;
    self
  }

  pub fn index(mut self, index: u32) -> Self {
    self.def.index = index;
    self
  }

  pub fn add_feature(mut self, feature: ServerDeviceFeature) -> Self {
    self.def.features.insert(feature.index(), feature);
    self
  }

  pub fn replace_feature(mut self, feature: ServerDeviceFeature) -> Self {
    if let Some((_, f)) = self
      .def
      .features
      .iter_mut()
      .find(|(_, x)| x.id() == feature.id())
    {
      *f = feature;
    }
    self
  }

  pub fn finish(self) -> ServerDeviceDefinition {
    self.def
  }
}
