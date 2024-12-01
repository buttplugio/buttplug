// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{core::message::{
  ActuatorType, ButtplugActuatorFeatureMessageType, ButtplugSensorFeatureMessageType, DeviceFeature, FeatureType, SensorType
}, server::message::{v1::NullDeviceMessageAttributesV1, v2::{ClientDeviceMessageAttributesV2, GenericDeviceMessageAttributesV2, RawDeviceMessageAttributesV2, SensorDeviceMessageAttributesV2}}};
use getset::{Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::ops::RangeInclusive;

// This will look almost exactly like ServerDeviceMessageAttributes. However, it will only contain
// information we want the client to know, i.e. step counts versus specific step ranges. This is
// what will be sent to the client as part of DeviceAdded/DeviceList messages. It should not be used
// for outside configuration/serialization, rather it should be a subset of that information.
//
// For many messages, client and server configurations may be exactly the same. If they are not,
// then we denote this by prefixing the type with Client/Server. Server attributes will usually be
// hosted in the server/device/configuration module.
#[derive(Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters)]
#[cfg_attr(feature = "serialize-json", derive(Serialize, Deserialize))]
pub struct ClientDeviceMessageAttributesV3 {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) scalar_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) rotate_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) linear_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,

  // Sensor Messages
  #[getset(get = "pub")]
  #[serde(rename = "SensorReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  #[serde(skip_deserializing)]
  pub(in crate::server::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_read_cmd: Option<RawDeviceMessageAttributesV2>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_write_cmd: Option<RawDeviceMessageAttributesV2>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::server::message) raw_subscribe_cmd: Option<RawDeviceMessageAttributesV2>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip_serializing)]
  pub(in crate::server::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip_serializing)]
  pub(in crate::server::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

pub fn vibrate_cmd_from_scalar_cmd(
  attributes_vec: &[ClientGenericDeviceMessageAttributesV3],
) -> GenericDeviceMessageAttributesV2 {
  let mut feature_count = 0u32;
  let mut step_count = vec![];
  let mut features = vec![];
  for attr in attributes_vec {
    if *attr.actuator_type() == ActuatorType::Vibrate {
      feature_count += 1;
      step_count.push(*attr.step_count());
      features.push(attr.feature().clone());
    }
  }
  GenericDeviceMessageAttributesV2 {
    feature_count,
    step_count,
    features,
  }
}

impl From<ClientDeviceMessageAttributesV3> for ClientDeviceMessageAttributesV2 {
  fn from(other: ClientDeviceMessageAttributesV3) -> Self {
    Self {
      vibrate_cmd: other
        .scalar_cmd()
        .as_ref()
        .map(|x| vibrate_cmd_from_scalar_cmd(x))
        .filter(|x| x.feature_count() != 0),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      battery_level_cmd: {
        if let Some(sensor_info) = other.sensor_read_cmd() {
          if let Some(attr) = sensor_info
            .iter()
            .find(|x| *x.sensor_type() == SensorType::Battery)
          {
            Some(SensorDeviceMessageAttributesV2::new(attr.feature()))
          } else {
            None
          }
        } else {
          None
        }
      },
      rssi_level_cmd: {
        if let Some(sensor_info) = other.sensor_read_cmd() {
          if let Some(attr) = sensor_info
            .iter()
            .find(|x| *x.sensor_type() == SensorType::RSSI)
          {
            Some(SensorDeviceMessageAttributesV2::new(attr.feature()))
          } else {
            None
          }
        } else {
          None
        }
      },
      stop_device_cmd: other.stop_device_cmd().clone(),
      raw_read_cmd: other.raw_read_cmd().clone(),
      raw_write_cmd: other.raw_write_cmd().clone(),
      raw_subscribe_cmd: other.raw_subscribe_cmd().clone(),
      raw_unsubscribe_cmd: other.raw_subscribe_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
    }
  }
}

impl ClientDeviceMessageAttributesV3 {
  pub fn raw_unsubscribe_cmd(&self) -> &Option<RawDeviceMessageAttributesV2> {
    self.raw_subscribe_cmd()
  }

  pub fn finalize(&mut self) {
    if let Some(scalar_attrs) = &mut self.scalar_cmd {
      for (i, attr) in scalar_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_read_attrs) = &mut self.sensor_read_cmd {
      for (i, attr) in sensor_read_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_subscribe_attrs) = &mut self.sensor_subscribe_cmd {
      for (i, attr) in sensor_subscribe_attrs.iter_mut().enumerate() {
        attr.index = i as u32;
      }
    }
  }
}

fn unspecified_feature() -> String {
  "N/A".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientGenericDeviceMessageAttributesV3 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  #[serde(default = "unspecified_feature")]
  pub(in crate::server::message) feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  pub(in crate::server::message) actuator_type: ActuatorType,
  #[serde(rename = "StepCount")]
  #[getset(get = "pub")]
  pub(in crate::server::message) step_count: u32,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  pub(in crate::server::message) index: u32,
  // Matching device feature for this attribute. Do not serialize or deserialize this, it's not part
  // of this version of the protocol, only use it for comparison when doing message conversion.
  #[getset(get = "pub")]
  #[serde(skip)]
  pub(in crate::server::message) feature: DeviceFeature,
}

impl From<Vec<ClientGenericDeviceMessageAttributesV3>> for GenericDeviceMessageAttributesV2 {
  fn from(attributes_vec: Vec<ClientGenericDeviceMessageAttributesV3>) -> Self {
    Self {
      feature_count: attributes_vec.len() as u32,
      step_count: attributes_vec.iter().map(|x| *x.step_count()).collect(),
      features: attributes_vec.iter().map(|x| x.feature().clone()).collect(),
    }
  }
}

impl ClientGenericDeviceMessageAttributesV3 {
  pub fn new(
    feature_descriptor: &str,
    step_count: u32,
    actuator_type: ActuatorType,
    feature: &DeviceFeature,
  ) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      actuator_type,
      step_count,
      feature: feature.clone(),
      index: 0,
    }
  }
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct SensorDeviceMessageAttributesV3 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  pub(in crate::server::message) feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  pub(in crate::server::message) sensor_type: SensorType,
  #[getset(get = "pub")]
  #[serde(rename = "SensorRange", serialize_with = "range_sequence_serialize")]
  pub(in crate::server::message) sensor_range: Vec<RangeInclusive<i32>>,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  pub(in crate::server::message) index: u32,
  // Matching device feature for this attribute. Do not serialize or deserialize this, it's not part
  // of this version of the protocol, only use it for comparison when doing message conversion.
  #[getset(get = "pub")]
  #[serde(skip)]
  pub(in crate::server::message) feature: DeviceFeature,
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
        index: 0,
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

impl TryFrom<DeviceFeature> for SensorDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(sensor) = value.sensor() {
      Ok(Self {
        feature_descriptor: value.description().to_owned(),
        sensor_type: (*value.feature_type()).try_into()?,
        sensor_range: sensor.value_range().clone(),
        feature: value.clone(),
        index: 0,
      })
    } else {
      Err("Device Feature does not expose a sensor.".to_owned())
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
            !(*message_type == ButtplugActuatorFeatureMessageType::LevelCmd
              && *x.feature_type() == FeatureType::RotateWithDirection)
              && actuator.messages().contains(message_type)
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
            actuator
              .messages()
              .contains(&ButtplugActuatorFeatureMessageType::LevelCmd)
              && *x.feature_type() == FeatureType::RotateWithDirection
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
