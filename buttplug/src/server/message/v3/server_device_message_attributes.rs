// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::message::{
    ActuatorType,
    ButtplugActuatorFeatureMessageType,
    ButtplugSensorFeatureMessageType,
    FeatureType,
    SensorType,
  },
  server::message::{
    server_device_feature::ServerDeviceFeature,
    v1::NullDeviceMessageAttributesV1,
    v2::RawDeviceMessageAttributesV2,
  },
};
use getset::{Getters, MutGetters, Setters};
use std::ops::RangeInclusive;

#[derive(Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters)]
#[getset(get = "pub")]
pub struct ServerDeviceMessageAttributesV3 {
  // Generic commands
  pub(in crate::server::message) scalar_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,
  pub(in crate::server::message) rotate_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,
  pub(in crate::server::message) linear_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,

  // Sensor Messages
  pub(in crate::server::message) sensor_read_cmd:
    Option<Vec<ServerSensorDeviceMessageAttributesV3>>,
  pub(in crate::server::message) sensor_subscribe_cmd:
    Option<Vec<ServerSensorDeviceMessageAttributesV3>>,

  // StopDeviceCmd always exists
  pub(in crate::server::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Raw commands are only added post-serialization
  pub(in crate::server::message) raw_read_cmd: Option<RawDeviceMessageAttributesV2>,
  pub(in crate::server::message) raw_write_cmd: Option<RawDeviceMessageAttributesV2>,
  pub(in crate::server::message) raw_subscribe_cmd: Option<RawDeviceMessageAttributesV2>,

  // Needed to load from config for fallback, but unused here.
  pub(in crate::server::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  pub(in crate::server::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, Setters)]
#[getset(get = "pub")]
pub struct ServerGenericDeviceMessageAttributesV3 {
  pub(in crate::server::message) feature_descriptor: String,
  pub(in crate::server::message) actuator_type: ActuatorType,
  pub(in crate::server::message) step_count: u32,
  pub(in crate::server::message) index: u32,
  pub(in crate::server::message) feature: ServerDeviceFeature,
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, Setters)]
#[getset(get = "pub")]
pub struct ServerSensorDeviceMessageAttributesV3 {
  pub(in crate::server::message) feature_descriptor: String,
  pub(in crate::server::message) sensor_type: SensorType,
  pub(in crate::server::message) sensor_range: Vec<RangeInclusive<i32>>,
  pub(in crate::server::message) index: u32,
  pub(in crate::server::message) feature: ServerDeviceFeature,
}

impl TryFrom<ServerDeviceFeature> for ServerGenericDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: ServerDeviceFeature) -> Result<Self, Self::Error> {
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

impl TryFrom<ServerDeviceFeature> for ServerSensorDeviceMessageAttributesV3 {
  type Error = String;
  fn try_from(value: ServerDeviceFeature) -> Result<Self, Self::Error> {
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

impl From<Vec<ServerDeviceFeature>> for ServerDeviceMessageAttributesV3 {
  fn from(features: Vec<ServerDeviceFeature>) -> Self {
    let actuator_filter = |message_type: &ButtplugActuatorFeatureMessageType| {
      let attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            // Carve out RotateCmd here
            !(*message_type == ButtplugActuatorFeatureMessageType::ValueCmd
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
      let attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            actuator
              .messages()
              .contains(&ButtplugActuatorFeatureMessageType::ValueWithParameterCmd)
              && *x.feature_type() == FeatureType::RotateWithDirection
          } else {
            false
          }
        })
        .map(|x| {
          // RotateWithDirection is a v4 Type, convert back to Rotate for v3
          let mut attr: ServerGenericDeviceMessageAttributesV3 = x.clone().try_into().unwrap();
          attr.actuator_type = ActuatorType::Rotate;
          attr
        })
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    let linear_attributes = {
      let attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            actuator
              .messages()
              .contains(&ButtplugActuatorFeatureMessageType::ValueWithParameterCmd)
              && *x.feature_type() == FeatureType::PositionWithDuration
          } else {
            false
          }
        })
        .map(|x| {
          // PositionWithDuration is a v4 Type, convert back to Position for v3
          let mut attr: ServerGenericDeviceMessageAttributesV3 = x.clone().try_into().unwrap();
          attr.actuator_type = ActuatorType::Position;
          attr
        })
        .collect();
      if !attrs.is_empty() {
        Some(attrs)
      } else {
        None
      }
    };

    let sensor_filter = |message_type| {
      let attrs: Vec<ServerSensorDeviceMessageAttributesV3> = features
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
      scalar_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::ValueCmd),
      rotate_cmd: rotate_attributes,
      linear_cmd: linear_attributes,
      sensor_read_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorReadCmd),
      sensor_subscribe_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd),
      raw_read_cmd: raw_attrs.clone(),
      raw_write_cmd: raw_attrs.clone(),
      raw_subscribe_cmd: raw_attrs.clone(),
      ..Default::default()
    }
  }
}
