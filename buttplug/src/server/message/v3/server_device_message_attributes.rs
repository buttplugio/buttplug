// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
  core::message::{ActuatorType, SensorType},
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

impl From<Vec<ServerDeviceFeature>> for ServerDeviceMessageAttributesV3 {
  fn from(features: Vec<ServerDeviceFeature>) -> Self {
    let scalar_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(actuator_map) = feature.actuator() {
          for (actuator_type, actuator) in actuator_map {
            if ![
              ActuatorType::PositionWithDuration,
              ActuatorType::RotateWithDirection,
            ]
            .contains(actuator_type)
            {
              let actuator_type = *actuator_type;
              let step_limit = actuator.step_limit();
              let step_count = step_limit.end() - step_limit.start();
              let attrs = ServerGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count,
                feature: feature.clone(),
                index: 0,
              };
              actuator_vec.push(attrs)
            }
          }
        }
        actuator_vec
      })
      .collect();

    // We have to calculate rotation attributes seperately, since they're a combination of
    // feature type and message in >= v4.
    let rotate_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(actuator_map) = feature.actuator() {
          for (actuator_type, actuator) in actuator_map {
            if *actuator_type == ActuatorType::RotateWithDirection {
              let actuator_type = ActuatorType::Rotate;
              let step_limit = actuator.step_limit();
              let step_count = step_limit.end() - step_limit.start();
              let attrs = ServerGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count,
                feature: feature.clone(),
                index: 0,
              };
              actuator_vec.push(attrs)
            }
          }
        }
        actuator_vec
      })
      .collect();

    let linear_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(actuator_map) = feature.actuator() {
          for (actuator_type, actuator) in actuator_map {
            if *actuator_type == ActuatorType::PositionWithDuration {
              let actuator_type = ActuatorType::Position;
              let step_limit = actuator.step_limit();
              let step_count = step_limit.end() - step_limit.start();
              let attrs = ServerGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count,
                feature: feature.clone(),
                index: 0,
              };
              actuator_vec.push(attrs)
            }
          }
        }
        actuator_vec
      })
      .collect();

    let sensor_filter = {
      let attrs: Vec<ServerSensorDeviceMessageAttributesV3> = features
        .iter()
        .map(|feature| {
          let mut sensor_vec = vec![];
          if let Some(sensor_map) = feature.sensor() {
            for (sensor_type, sensor) in sensor_map {
              // Only convert Battery backwards. Other sensors weren't really built for v3 and we
              // never recommended using them or implemented much for them.
              if *sensor_type == SensorType::Battery {
                sensor_vec.push(ServerSensorDeviceMessageAttributesV3 {
                  feature_descriptor: feature.description().to_owned(),
                  sensor_type: *sensor_type,
                  sensor_range: sensor.value_range().clone(),
                  feature: feature.clone(),
                  index: 0,
                });
              }
            }
          }
          sensor_vec
        })
        .flatten()
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
      scalar_cmd: if scalar_attrs.is_empty() {
        None
      } else {
        Some(scalar_attrs)
      },
      rotate_cmd: if rotate_attrs.is_empty() {
        None
      } else {
        Some(rotate_attrs)
      },
      linear_cmd: if linear_attrs.is_empty() {
        None
      } else {
        Some(linear_attrs)
      },
      sensor_read_cmd: sensor_filter,
      sensor_subscribe_cmd: None,
      raw_read_cmd: raw_attrs.clone(),
      raw_write_cmd: raw_attrs.clone(),
      raw_subscribe_cmd: raw_attrs.clone(),
      ..Default::default()
    }
  }
}
