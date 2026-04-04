// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::v1::NullDeviceMessageAttributesV1;
use buttplug_core::message::{InputType, OutputType};
use buttplug_server_device_config::{
  ServerDeviceFeature,
  ServerDeviceFeatureInput,
  ServerDeviceFeatureOutput,
};

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
        feature
          .output
          .iter()
          .filter(|output| !matches!(output, ServerDeviceFeatureOutput::HwPositionWithDuration(_)))
          .map(|output| {
            let actuator_type = output.output_type();
            let step_count = match output {
              ServerDeviceFeatureOutput::Position(p) => p.value.step_count(),
              _ => output.as_value_properties().unwrap().value.step_count(),
            };
            ServerGenericDeviceMessageAttributesV3 {
              feature_descriptor: feature.description.clone(),
              actuator_type,
              step_count,
              feature: feature.clone(),
              index: 0,
            }
          })
          .collect::<Vec<_>>()
      })
      .collect();

    // We have to calculate rotation attributes seperately, since they're a combination of
    // feature type and message in >= v4.
    let rotate_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(ServerDeviceFeatureOutput::Rotate(r)) = feature.get_output(OutputType::Rotate)
          && r.value.base.start() < 0
        {
          actuator_vec.push(ServerGenericDeviceMessageAttributesV3 {
            feature_descriptor: feature.description.clone(),
            actuator_type: OutputType::Rotate,
            step_count: r.value.step_count(),
            feature: feature.clone(),
            index: 0,
          });
        }
        actuator_vec
      })
      .collect();

    let linear_attrs: Vec<ServerGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(ServerDeviceFeatureOutput::HwPositionWithDuration(p)) =
          feature.get_output(OutputType::HwPositionWithDuration)
        {
          actuator_vec.push(ServerGenericDeviceMessageAttributesV3 {
            feature_descriptor: feature.description.clone(),
            actuator_type: OutputType::Position,
            step_count: p.value.step_count(),
            feature: feature.clone(),
            index: 0,
          });
        }
        actuator_vec
      })
      .collect();

    let sensor_filter = {
      let attrs: Vec<ServerSensorDeviceMessageAttributesV3> = features
        .iter()
        .map(|feature| {
          let mut sensor_vec = vec![];
          if let Some(ServerDeviceFeatureInput::Battery(battery)) =
            feature.get_input(InputType::Battery)
          {
            // Only convert Battery backwards. Other sensors weren't really built for v3 and we
            // never recommended using them or implemented much for them.
            sensor_vec.push(ServerSensorDeviceMessageAttributesV3 {
              feature_descriptor: feature.description.clone(),
              sensor_type: InputType::Battery,
              sensor_range: battery.value.iter().map(|r| r.start()..=r.end()).collect(),
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
