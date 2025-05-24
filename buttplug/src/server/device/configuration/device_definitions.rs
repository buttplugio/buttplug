use getset::{CopyGetters, Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  core::message::{
    ButtplugActuatorFeatureMessageType,
    ButtplugRawFeatureMessageType,
    ButtplugSensorFeatureMessageType,
    Endpoint,
    FeatureType,
  },
  server::message::{server_device_feature::ServerDeviceFeature, ButtplugDeviceMessageType},
};

#[derive(Debug, Clone, Getters)]
#[getset(get = "pub")]
pub struct BaseDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  /// Message attributes for this device instance.
  features: Vec<ServerDeviceFeature>,
  id: Uuid,
}

impl BaseDeviceDefinition {
  /// Create a new instance
  pub fn new(name: &str, id: &Uuid, features: &[ServerDeviceFeature]) -> Self {
    Self {
      name: name.to_owned(),
      features: features.into(),
      id: *id,
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct UserDeviceDefinition {
  /// Given name of the device this instance represents.
  name: String,
  id: Uuid,
  #[serde(skip_serializing_if = "Option::is_none", rename="base-id")]
  base_id: Option<Uuid>,
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
    features: &[ServerDeviceFeature],
    user_config: &UserDeviceCustomization,
  ) -> Self {
    Self {
      name: name.to_owned(),
      id: id.to_owned(),
      base_id: base_id.to_owned(),
      features: features.into(),
      user_config: user_config.clone(),
    }
  }

  pub fn new_from_base_definition(def: &BaseDeviceDefinition, index: u32) -> Self {
    Self {
      name: def.name().clone(),
      id: Uuid::new_v4(),
      base_id: Some(*def.id()),
      features: def.features().clone(),
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

  // Return true if any feature on this device handles this message. We'll deal with the actual
  // feature indexing when the message itself is handled.
  pub fn allows_message(&self, msg_type: &ButtplugDeviceMessageType) -> bool {
    for feature in &self.features {
      if let Ok(actuator_msg_type) = ButtplugActuatorFeatureMessageType::try_from(*msg_type) {
        if let Some(actuator) = feature.actuator() {
          debug!("{:?}", actuator);
          if actuator.messages().contains(&actuator_msg_type) {
            return true;
          }
          if *msg_type == ButtplugDeviceMessageType::RotateCmd
            && actuator
              .messages()
              .contains(&ButtplugActuatorFeatureMessageType::ValueCmd)
            && *feature.feature_type() == FeatureType::RotateWithDirection
          {
            return true;
          }
        }
      } else if let Ok(sensor_msg_type) = ButtplugSensorFeatureMessageType::try_from(*msg_type) {
        if let Some(sensor) = feature.sensor() {
          if sensor.messages().contains(&sensor_msg_type) {
            return true;
          }
        }
      } else if ButtplugRawFeatureMessageType::try_from(*msg_type).is_ok()
        && feature.raw().is_some()
      {
        return true;
      }
    }
    false
  }
}
