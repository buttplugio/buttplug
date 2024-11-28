// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{ButtplugDeviceMessageType, Endpoint},
};
use getset::{Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{collections::HashSet, ops::RangeInclusive};
use uuid::Uuid;

use super::{
  ActuatorType, ButtplugActuatorFeatureMessageType, ButtplugSensorFeatureMessageType, ClientDeviceMessageAttributesV1, ClientDeviceMessageAttributesV2, ClientDeviceMessageAttributesV3, ClientGenericDeviceMessageAttributesV3, RawDeviceMessageAttributesV2, SensorDeviceMessageAttributesV3, SensorType
};

#[derive(Debug, Default, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureType {
  #[default]
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  // Two Direction Rotation Speed
  RotateWithDirection,
  Oscillate,
  Constrict,
  Inflate,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
  // Sensor Types
  Battery,
  RSSI,
  Button,
  Pressure,
  // Currently unused but possible sensor features:
  // Temperature,
  // Accelerometer,
  // Gyro,
  //
  // Raw Feature, for when raw messages are on
  Raw,
}

impl From<ActuatorType> for FeatureType {
  fn from(value: ActuatorType) -> Self {
    match value {
      ActuatorType::Unknown => FeatureType::Unknown,
      ActuatorType::Vibrate => FeatureType::Vibrate,
      ActuatorType::Rotate => FeatureType::Rotate,
      ActuatorType::RotateWithDirection => FeatureType::RotateWithDirection,
      ActuatorType::Oscillate => FeatureType::Oscillate,
      ActuatorType::Constrict => FeatureType::Constrict,
      ActuatorType::Inflate => FeatureType::Inflate,
      ActuatorType::Position => FeatureType::Position,
    }
  }
}

impl From<SensorType> for FeatureType {
  fn from(value: SensorType) -> Self {
    match value {
      SensorType::Unknown => FeatureType::Unknown,
      SensorType::Battery => FeatureType::Battery,
      SensorType::RSSI => FeatureType::RSSI,
      SensorType::Button => FeatureType::Button,
      SensorType::Pressure => FeatureType::Pressure,
    }
  }
}

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
pub struct DeviceFeature {
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
  #[serde(skip_serializing)]
  id: Uuid,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "base-id")]
  #[serde(skip_serializing)]
  base_id: Option<Uuid>,
}

impl DeviceFeature {
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
      id: id.clone(),
      base_id: base_id.clone(),
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
      base_id: None
    }
  }
}

fn range_serialize<S>(range: &RangeInclusive<u32>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let mut seq = serializer.serialize_seq(Some(2))?;
  seq.serialize_element(&range.start())?;
  seq.serialize_element(&range.end())?;
  seq.end()
}

fn range_i32_serialize<S>(range: &RangeInclusive<i32>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let mut seq = serializer.serialize_seq(Some(2))?;
  seq.serialize_element(&range.start())?;
  seq.serialize_element(&range.end())?;
  seq.end()
}

fn range_sequence_serialize<S>(
  range_vec: &Vec<RangeInclusive<i32>>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  let mut seq = serializer.serialize_seq(Some(range_vec.len()))?;
  for range in range_vec {
    seq.serialize_element(&vec![*range.start(), *range.end()])?;
  }
  seq.end()
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
pub struct DeviceFeatureActuatorSerialized {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  #[serde(serialize_with = "range_i32_serialize")]
  step_range: RangeInclusive<i32>,
  // This doesn't exist in base configs, so when we load these from the base config file, we'll just
  // copy the step_range value.
  #[getset(get = "pub")]
  #[serde(rename = "step-limit")]
  #[serde(default)]
  step_limit: Option<RangeInclusive<u32>>,
  #[getset(get = "pub")]
  #[serde(rename = "messages")]
  messages: HashSet<ButtplugActuatorFeatureMessageType>,
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize)]
#[serde(from = "DeviceFeatureActuatorSerialized")]
pub struct DeviceFeatureActuator {
  #[getset(get = "pub")]
  #[serde(rename = "step-range")]
  #[serde(serialize_with = "range_i32_serialize")]
  step_range: RangeInclusive<i32>,
  // This doesn't exist in base configs, so when we load these from the base config file, we'll just
  // copy the step_range value.
  #[getset(get = "pub")]
  #[serde(rename = "step-limit")]
  #[serde(serialize_with = "range_serialize")]
  step_limit: RangeInclusive<u32>,
  #[getset(get = "pub")]
  #[serde(rename = "messages")]
  messages: HashSet<ButtplugActuatorFeatureMessageType>,
}

impl From<DeviceFeatureActuatorSerialized> for DeviceFeatureActuator {
  fn from(value: DeviceFeatureActuatorSerialized) -> Self {
    Self {
      step_range: value.step_range.clone(),
      step_limit: value.step_limit.unwrap_or(RangeInclusive::new(0, value.step_range.end().abs() as u32)),
      messages: value.messages,
    }
  }
}

impl DeviceFeatureActuator {
  pub fn new(
    step_range: &RangeInclusive<i32>,
    step_limit: &RangeInclusive<u32>,
    messages: &HashSet<ButtplugActuatorFeatureMessageType>,
  ) -> Self {
    Self {
      step_range: step_range.clone(),
      step_limit: step_limit.clone(),
      messages: messages.clone(),
    }
  }

  pub fn is_valid(&self) -> Result<(), ButtplugDeviceError> {
    if self.step_range.is_empty() || self.step_range.start() > self.step_range.end() {
      Err(ButtplugDeviceError::DeviceConfigurationError(
        "Step range out of order, must be start <= x <= end.".to_string(),
      ))
    } else if self.step_limit.is_empty() || self.step_limit.start() > self.step_limit.end() {
      Err(ButtplugDeviceError::DeviceConfigurationError(
        "Step limit out of order, must be start <= x <= end.".to_string(),
      ))
    } else {
      Ok(())
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureSensor {
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "value-range")]
  #[serde(serialize_with = "range_sequence_serialize")]
  value_range: Vec<RangeInclusive<i32>>,
  #[getset(get = "pub")]
  #[serde(rename = "messages")]
  messages: HashSet<ButtplugSensorFeatureMessageType>,
}

impl DeviceFeatureSensor {
  pub fn new(
    value_range: &Vec<RangeInclusive<i32>>,
    messages: &HashSet<ButtplugSensorFeatureMessageType>,
  ) -> Self {
    Self {
      value_range: value_range.clone(),
      messages: messages.clone(),
    }
  }
}

#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct DeviceFeatureRaw {
  #[getset(get = "pub")]
  #[serde(rename = "Endpoints")]
  endpoints: Vec<Endpoint>,
  #[getset(get = "pub")]
  #[serde(rename = "Messages")]
  messages: HashSet<ButtplugDeviceMessageType>,
}

impl DeviceFeatureRaw {
  pub fn new(endpoints: &[Endpoint]) -> Self {
    Self {
      endpoints: endpoints.into(),
      messages: HashSet::from_iter(
        [
          ButtplugDeviceMessageType::RawReadCmd,
          ButtplugDeviceMessageType::RawWriteCmd,
          ButtplugDeviceMessageType::RawSubscribeCmd,
          ButtplugDeviceMessageType::RawUnsubscribeCmd,
        ]
        .iter()
        .cloned(),
      ),
    }
  }
}

/// TryFrom for Buttplug Device Messages that need to use a device feature definition to convert
pub(crate) trait TryFromDeviceAttributes<T> where Self: Sized {
  fn try_from_device_attributes(msg: T, features: &LegacyDeviceAttributes) -> Result<Self, ButtplugError>;
}

impl TryFrom<DeviceFeature> for SensorDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(sensor) = value.sensor() {
      Ok(Self {
        feature_descriptor: value.description().to_owned(),
        sensor_type: (*value.feature_type()).try_into()?,
        sensor_range: sensor.value_range().clone(),
        feature: value.clone(),
        index: 0
      })
    } else {
      Err("Device Feature does not expose a sensor.".to_owned())
    }
  }
}

impl TryFrom<DeviceFeature> for ClientGenericDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(actuator) = value.actuator() {
      let actuator_type = (*value.feature_type()).try_into()?;
      let step_limit = actuator.step_limit();
      let step_count = step_limit.end() - step_limit.start();
      let attrs = Self {
        feature_descriptor: value.description().to_owned(),
        actuator_type,
        step_count,
        feature: value.clone(),
        index: 0
      };
      Ok(attrs)
    } else {
      Err(
        "Cannot produce a GenericDeviceMessageAttribute from a feature with no actuator member"
          .to_string(),
      )
    }
  }
}

impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV3 {
  fn from(features: Vec<DeviceFeature>) -> Self {
    let actuator_filter = |message_type: &ButtplugActuatorFeatureMessageType| {
      let attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            // Carve out RotateCmd here
            !(*message_type == ButtplugActuatorFeatureMessageType::LevelCmd && *x.feature_type() == FeatureType::RotateWithDirection) && actuator.messages().contains(message_type)
          } else {
            false
          }
        })
        .map(|x| x.clone().try_into().unwrap())
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    // We have to calculate rotation attributes seperately, since they're a combination of
    // feature type and message in >= v4.
    let rotate_attributes = {
      let attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
      .iter()
      .filter(|x| {
        if let Some(actuator) = x.actuator() {
          actuator.messages().contains(&ButtplugActuatorFeatureMessageType::LevelCmd) && *x.feature_type() == FeatureType::RotateWithDirection
        } else {
          false
        }
      })
      .map(|x| x.clone().try_into().unwrap())
      .collect();
    if !attrs.is_empty() {
      Some(attrs)
    } else {
      None
    }
    };

    let sensor_filter = |message_type| {
      let attrs: Vec<SensorDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(sensor) = x.sensor() {
            sensor.messages().contains(message_type)
          } else {
            false
          }
        })
        .map(|x| x.clone().try_into().unwrap())
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    // Raw messages
    let raw_attrs = features
      .iter()
      .find(|f| f.raw().is_some())
      .map(|raw_feature| {
        RawDeviceMessageAttributesV2::new(raw_feature.raw().as_ref().unwrap().endpoints())
      });

    Self {
      scalar_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::LevelCmd),
      rotate_cmd: rotate_attributes,
      linear_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::LinearCmd),
      sensor_read_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorReadCmd),
      sensor_subscribe_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd),
      raw_read_cmd: raw_attrs.clone(),
      raw_write_cmd: raw_attrs.clone(),
      raw_subscribe_cmd: raw_attrs.clone(),
      ..Default::default()
    }
  }
}

impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV2 {
  fn from(value: Vec<DeviceFeature>) -> Self {
      ClientDeviceMessageAttributesV3::from(value).into()
  }
}

impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV1 {
  fn from(value: Vec<DeviceFeature>) -> Self {
      ClientDeviceMessageAttributesV2::from(ClientDeviceMessageAttributesV3::from(value)).into()
  }
}

#[derive(Debug, Getters, Clone)]
pub(crate) struct LegacyDeviceAttributes {
  /*  #[getset(get = "pub")]
  attrs_v1: ClientDeviceMessageAttributesV1,
  */
  #[getset(get = "pub")]
  attrs_v2: ClientDeviceMessageAttributesV2,
  #[getset(get = "pub")]
  attrs_v3: ClientDeviceMessageAttributesV3,
  #[getset(get = "pub")]
  features: Vec<DeviceFeature>
}

impl LegacyDeviceAttributes {
  pub fn new(features: &Vec<DeviceFeature>) -> Self {
    Self {
      attrs_v3: ClientDeviceMessageAttributesV3::from(features.clone()),
      attrs_v2: ClientDeviceMessageAttributesV2::from(features.clone()),
      /*
      attrs_v1: ClientDeviceMessageAttributesV1::from(features.clone()),
      */
      features: features.clone()
    }
  }
}
