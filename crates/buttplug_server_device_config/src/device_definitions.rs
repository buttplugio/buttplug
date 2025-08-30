use getset::{CopyGetters, Getters};
use uuid::Uuid;

use super::device_feature::{
  ServerDeviceFeature,
};
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
  features: Vec<ServerDeviceFeature>
}

pub struct ServerDeviceDefinitionBuilder {
  def: ServerDeviceDefinition,
}

impl ServerDeviceDefinitionBuilder {
  pub fn new(name: &str, id: &Uuid) -> Self {
    Self {
      def: ServerDeviceDefinition {
        name: name.to_owned(),
        id: id.clone(),
        base_id: None,
        protocol_variant: None,
        message_gap_ms: None,
        display_name: None,
        allow: false,
        deny: false,
        index: 0,
        features: vec!()
      },
    }
  }

  pub fn base_id(&mut self, id: &Uuid) -> &mut Self {
    self.def.base_id = Some(id.clone());
    self
  }

  pub fn display_name(&mut self, name: &str) -> &mut Self {
    self.def.display_name = Some(name.to_owned());
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

  pub fn allow(&mut self) -> &mut Self {
    self.def.allow = true;
    self
  }

  pub fn deny(&mut self) -> &mut Self {
    self.def.deny = true;
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

  pub fn finish(self) -> ServerDeviceDefinition {
    self.def
  }
}
