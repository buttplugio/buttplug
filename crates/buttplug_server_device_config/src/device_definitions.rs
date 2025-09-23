use getset::{CopyGetters, Getters};
use uuid::Uuid;

use super::server_device_feature::ServerDeviceFeature;
#[derive(Debug, Clone, Getters, CopyGetters)]
pub struct ServerDeviceDefinition {
  #[getset(get = "pub")]
  /// Given name of the device this instance represents.
  name: String,
  #[getset(get_copy = "pub")]
  id: Uuid,
  #[getset(get_copy = "pub")]
  base_id: Option<Uuid>,
  #[getset(get = "pub")]
  protocol_variant: Option<String>,
  #[getset(get_copy = "pub")]
  message_gap_ms: Option<u32>,
  #[getset(get = "pub")]
  display_name: Option<String>,
  #[getset(get_copy = "pub")]
  allow: bool,
  #[getset(get_copy = "pub")]
  deny: bool,
  #[getset(get_copy = "pub")]
  index: u32,
  #[getset(get = "pub")]
  features: Vec<ServerDeviceFeature>,
}

#[derive(Debug)]
pub struct ServerDeviceDefinitionBuilder {
  def: ServerDeviceDefinition,
}

impl ServerDeviceDefinitionBuilder {
  pub fn new(name: &str, id: &Uuid) -> Self {
    Self {
      def: ServerDeviceDefinition {
        name: name.to_owned(),
        id: *id,
        base_id: None,
        protocol_variant: None,
        message_gap_ms: None,
        display_name: None,
        allow: false,
        deny: false,
        index: 0,
        features: vec![],
      },
    }
  }

  // Used to create new user definitions from a base definition.
  pub fn from_base(value: &ServerDeviceDefinition, id: Uuid, with_features: bool) -> Self {
    let mut value = value.clone();
    value.base_id = Some(value.id);
    value.id = id;
    if with_features {
      value.features = value.features().iter().map(|x| x.as_new_user_feature()).collect();
    } else {
      value.features = vec!();
    }
    ServerDeviceDefinitionBuilder { def: value }
  }

  pub fn from_user(value: &ServerDeviceDefinition) -> Self {
    ServerDeviceDefinitionBuilder { def: value.clone() }
  }

  pub fn id(&mut self, id: Uuid) -> &mut Self {
    self.def.id = id;
    self
  }

  pub fn base_id(&mut self, id: Uuid) -> &mut Self {
    self.def.base_id = Some(id);
    self
  }

  pub fn display_name(&mut self, name: &Option<String>) -> &mut Self {
    self.def.display_name = name.clone();
    self
  }

  pub fn protocol_variant(&mut self, variant: &str) -> &mut Self {
    self.def.protocol_variant = Some(variant.to_owned());
    self
  }

  pub fn message_gap_ms(&mut self, gap: u32) -> &mut Self {
    self.def.message_gap_ms = Some(gap);
    self
  }

  pub fn allow(&mut self, allow: bool) -> &mut Self {
    self.def.allow = allow;
    self
  }

  pub fn deny(&mut self, deny: bool) -> &mut Self {
    self.def.deny = deny;
    self
  }

  pub fn index(&mut self, index: u32) -> &mut Self {
    self.def.index = index;
    self
  }

  pub fn add_feature(&mut self, feature: &ServerDeviceFeature) -> &mut Self {
    self.def.features.push(feature.clone());
    self
  }

  pub fn replace_feature(&mut self, feature: &ServerDeviceFeature) -> &mut Self {
    if let Some(f) = self.def.features.iter_mut().find(|x| x.id() == feature.id()) {
      *f = feature.clone();
    }
    self
  }

  pub fn finish(&self) -> ServerDeviceDefinition {
    self.def.clone()
  }
}
