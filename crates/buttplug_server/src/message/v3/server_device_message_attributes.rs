// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::v1::NullDeviceMessageAttributesV1;
use buttplug_core::message::{InputType, OutputType};
use buttplug_server_device_config::ServerDeviceFeature;

use getset::{Getters, MutGetters, Setters};
use std::ops::RangeInclusive;

#[derive(Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters)]
#[getset(get = "pub")]
pub struct ServerDeviceMessageAttributesV3 {
  // Generic commands
  pub(in crate::message) scalar_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,
  pub(in crate::message) rotate_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,
  pub(in crate::message) linear_cmd: Option<Vec<ServerGenericDeviceMessageAttributesV3>>,

  // Sensor Messages
  pub(in crate::message) sensor_read_cmd: Option<Vec<ServerSensorDeviceMessageAttributesV3>>,
  pub(in crate::message) sensor_subscribe_cmd: Option<Vec<ServerSensorDeviceMessageAttributesV3>>,

  // StopDeviceCmd always exists
  pub(in crate::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Needed to load from config for fallback, but unused here.
  pub(in crate::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  pub(in crate::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, Setters)]
#[getset(get = "pub")]
pub struct ServerGenericDeviceMessageAttributesV3 {
  pub(in crate::message) feature_descriptor: String,
  pub(in crate::message) actuator_type: OutputType,
  pub(in crate::message) step_count: u32,
  pub(in crate::message) index: u32,
  pub(in crate::message) feature: ServerDeviceFeature,
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, Setters)]
#[getset(get = "pub")]
pub struct ServerSensorDeviceMessageAttributesV3 {
  pub(in crate::message) feature_descriptor: String,
  pub(in crate::message) sensor_type: InputType,
  pub(in crate::message) sensor_range: Vec<RangeInclusive<i32>>,
  pub(in crate::message) index: u32,
  pub(in crate::message) feature: ServerDeviceFeature,
}

impl From<Vec<ServerDeviceFeature>> for ServerDeviceMessageAttributesV3 {
  fn from(features: Vec<ServerDeviceFeature>) -> Self {
    let scalar_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(output_map) = feature.output() {
          let mut create_attribute = |actuator_type, step_count| {
            let actuator_type = actuator_type;
            let attrs = ServerGenericDeviceMessageAttributesV3 {
              feature_descriptor: feature.description().to_owned(),
              actuator_type,
              step_count,
              feature: feature.clone(),
              index: 0,
            };
            actuator_vec.push(attrs)
          };
          // TODO oh come on just make a fucking iterator here. At least, once we figure out the
          // unifying trait we can use to make an iterator on this.
          if let Some(attr) = output_map.constrict().as_ref() {
            create_attribute(OutputType::Constrict, attr.value().step_count())
          }
          if let Some(attr) = output_map.oscillate().as_ref() {
            create_attribute(OutputType::Oscillate, attr.value().step_count())
          }
          if let Some(attr) = output_map.position().as_ref() {
            create_attribute(OutputType::Position, attr.value().step_count())
          }
          if let Some(attr) = output_map.rotate().as_ref() {
            create_attribute(OutputType::Rotate, attr.value().step_count())
          }
          if let Some(attr) = output_map.temperature().as_ref() {
            create_attribute(OutputType::Temperature, attr.value().step_count())
          }
          if let Some(attr) = output_map.led().as_ref() {
            create_attribute(OutputType::Led, attr.value().step_count())
          }
          if let Some(attr) = output_map.vibrate().as_ref() {
            create_attribute(OutputType::Vibrate, attr.value().step_count())
          }
          if let Some(attr) = output_map.spray().as_ref() {
            create_attribute(OutputType::Spray, attr.value().step_count())
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
        if let Some(output_map) = feature.output()
          && let Some(actuator) = output_map.rotate()
          && *actuator.value().base().start() < 0
        {
          let actuator_type = OutputType::Rotate;
          let step_count = actuator.value().step_count();
          let attrs = ServerGenericDeviceMessageAttributesV3 {
            feature_descriptor: feature.description().to_owned(),
            actuator_type,
            step_count,
            feature: feature.clone(),
            index: 0,
          };
          actuator_vec.push(attrs)
        }
        actuator_vec
      })
      .collect();

    let linear_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(output_map) = feature.output()
          && let Some(actuator) = output_map.hw_position_with_duration()
        {
          let actuator_type = OutputType::Position;
          let step_count = actuator.value().step_count();
          let attrs = ServerGenericDeviceMessageAttributesV3 {
            feature_descriptor: feature.description().to_owned(),
            actuator_type,
            step_count,
            feature: feature.clone(),
            index: 0,
          };
          actuator_vec.push(attrs)
        }
        actuator_vec
      })
      .collect();

    let sensor_filter = {
      let attrs: Vec<ServerSensorDeviceMessageAttributesV3> = features
        .iter()
        .map(|feature| {
          let mut sensor_vec = vec![];
          if let Some(sensor_map) = feature.input()
            && let Some(battery) = sensor_map.battery()
          {
            // Only convert Battery backwards. Other sensors weren't really built for v3 and we
            // never recommended using them or implemented much for them.
            sensor_vec.push(ServerSensorDeviceMessageAttributesV3 {
              feature_descriptor: feature.description().to_owned(),
              sensor_type: InputType::Battery,
              sensor_range: battery.value().clone(),
              feature: feature.clone(),
              index: 0,
            });
          }
          sensor_vec
        })
        .flatten()
        .collect();
      if !attrs.is_empty() { Some(attrs) } else { None }
    };

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
      ..Default::default()
    }
  }
}
