// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugDeviceError,
  message::{
    DeviceFeature,
    DeviceFeatureActuator,
    DeviceFeatureRaw,
    DeviceFeatureSensor,
    Endpoint,
    FeatureType,
  },
};
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// This will look almost exactly like ServerDeviceFeature. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct ServerDeviceFeature {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(default)]
  description: String,
  #[getset(get = "pub")]
  #[serde(rename = "feature-type")]
  feature_type: FeatureType,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "actuator")]
  actuator: Option<DeviceFeatureActuator>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(rename = "sensor")]
  sensor: Option<DeviceFeatureSensor>,
  #[getset(get = "pub")]
  #[serde(skip)]
  raw: Option<DeviceFeatureRaw>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  id: Uuid,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "base-id", skip_serializing_if = "Option::is_none")]
  base_id: Option<Uuid>,
}

impl ServerDeviceFeature {
  pub fn new(
    description: &str,
    id: &Uuid,
    base_id: &Option<Uuid>,
    feature_type: FeatureType,
    actuator: &Option<DeviceFeatureActuator>,
    sensor: &Option<DeviceFeatureSensor>,
  ) -> Self {
    Self {
      description: description.to_owned(),
      feature_type,
      actuator: actuator.clone(),
      sensor: sensor.clone(),
      raw: None,
      id: *id,
      base_id: *base_id,
    }
  }

  pub fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    if let Some(actuator) = &self.actuator {
      actuator.is_valid()?;
    }
    Ok(())
  }

  pub fn new_raw_feature(endpoints: &[Endpoint]) -> Self {
    Self {
      description: "Raw Endpoints".to_owned(),
      feature_type: FeatureType::Raw,
      actuator: None,
      sensor: None,
      raw: Some(DeviceFeatureRaw::new(endpoints)),
      id: uuid::Uuid::new_v4(),
      base_id: None,
    }
  }
}

impl From<ServerDeviceFeature> for DeviceFeature {
  fn from(value: ServerDeviceFeature) -> Self {
    DeviceFeature::new(
      value.description(),
      *value.feature_type(),
      value.actuator(),
      value.sensor(),
      value.raw()
    )
  }
}
