// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::message::{
  v1::NullDeviceMessageAttributesV1,
  v2::{ClientDeviceMessageAttributesV2, GenericDeviceMessageAttributesV2},
};
use buttplug_core::message::{DeviceFeature, InputCommandType, InputType, OutputType};
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
#[derive(
  Clone, Debug, Default, PartialEq, Getters, MutGetters, Setters, Serialize, Deserialize,
)]
pub struct ClientDeviceMessageAttributesV3 {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) scalar_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) rotate_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) linear_cmd: Option<Vec<ClientGenericDeviceMessageAttributesV3>>,

  // Sensor Messages
  #[getset(get = "pub")]
  #[serde(rename = "SensorReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  pub(in crate::message) sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributesV3>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  #[serde(skip_deserializing)]
  pub(in crate::message) stop_device_cmd: NullDeviceMessageAttributesV1,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip_serializing)]
  pub(in crate::message) fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip_serializing)]
  pub(in crate::message) vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributesV1>,
}

pub fn vibrate_cmd_from_scalar_cmd(
  attributes_vec: &[ClientGenericDeviceMessageAttributesV3],
) -> GenericDeviceMessageAttributesV2 {
  let mut feature_count = 0u32;
  let mut step_count = vec![];
  for attr in attributes_vec {
    if *attr.actuator_type() == OutputType::Vibrate {
      feature_count += 1;
      step_count.push(*attr.step_count());
    }
  }
  GenericDeviceMessageAttributesV2 {
    feature_count,
    step_count,
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
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == InputType::Battery)
          {
            Some(NullDeviceMessageAttributesV1::default())
          } else {
            None
          }
        } else {
          None
        }
      },
      rssi_level_cmd: {
        if let Some(sensor_info) = other.sensor_read_cmd() {
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == InputType::Rssi)
          {
            Some(NullDeviceMessageAttributesV1::default())
          } else {
            None
          }
        } else {
          None
        }
      },
      stop_device_cmd: other.stop_device_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
    }
  }
}

impl ClientDeviceMessageAttributesV3 {
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
  pub(in crate::message) feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  pub(in crate::message) actuator_type: OutputType,
  #[serde(rename = "StepCount")]
  #[getset(get = "pub")]
  pub(in crate::message) step_count: u32,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  pub(in crate::message) index: u32,
}

impl From<Vec<ClientGenericDeviceMessageAttributesV3>> for GenericDeviceMessageAttributesV2 {
  fn from(attributes_vec: Vec<ClientGenericDeviceMessageAttributesV3>) -> Self {
    Self {
      feature_count: attributes_vec.len() as u32,
      step_count: attributes_vec.iter().map(|x| *x.step_count()).collect(),
    }
  }
}

impl ClientGenericDeviceMessageAttributesV3 {
  pub fn new(feature_descriptor: &str, step_count: u32, actuator_type: OutputType) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      actuator_type,
      step_count,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Getters, Setters)]
pub struct SensorDeviceMessageAttributesV3 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  pub(in crate::message) feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  pub(in crate::message) sensor_type: InputType,
  #[getset(get = "pub")]
  #[serde(rename = "SensorRange", serialize_with = "range_sequence_serialize")]
  pub(in crate::message) sensor_range: Vec<RangeInclusive<i32>>,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  pub(in crate::message) index: u32,
  // Matching device feature for this attribute. Do not serialize or deserialize this, it's not part
  // of this version of the protocol, only use it for comparison when doing message conversion.
  #[getset(get = "pub")]
  #[serde(skip)]
  pub(in crate::message) feature: DeviceFeature,
}

// This is an almost exact copy of the conversion we do for ServerDeviceFeature ->
// ServerDeviceMessageAttributesV3, but there's enough differences in the members that it's just
// easiest to mostly repeat here.
impl From<Vec<DeviceFeature>> for ClientDeviceMessageAttributesV3 {
  fn from(features: Vec<DeviceFeature>) -> Self {
    let scalar_attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(output_map) = feature.output() {
          for (actuator_type, actuator) in output_map {
            if ![
              OutputType::PositionWithDuration,
              OutputType::RotateWithDirection,
            ]
            .contains(actuator_type)
            {
              let actuator_type = *actuator_type;
              let attrs = ClientGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count: actuator.step_count(),
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
    let rotate_attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(output_map) = feature.output() {
          for (actuator_type, actuator) in output_map {
            if *actuator_type == OutputType::RotateWithDirection {
              let actuator_type = OutputType::Rotate;
              let attrs = ClientGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count: actuator.step_count(),
                index: 0,
              };
              actuator_vec.push(attrs)
            }
          }
        }
        actuator_vec
      })
      .collect();

    let linear_attrs: Vec<ClientGenericDeviceMessageAttributesV3> = features
      .iter()
      .flat_map(|feature| {
        let mut actuator_vec = vec![];
        if let Some(output_map) = feature.output() {
          for (actuator_type, actuator) in output_map {
            if *actuator_type == OutputType::PositionWithDuration {
              let actuator_type = OutputType::Position;
              let attrs = ClientGenericDeviceMessageAttributesV3 {
                feature_descriptor: feature.description().to_owned(),
                actuator_type,
                step_count: actuator.step_count(),
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
      let attrs: Vec<SensorDeviceMessageAttributesV3> = features
        .iter()
        .map(|feature| {
          let mut sensor_vec = vec![];
          if let Some(sensor_map) = feature.input() {
            for (sensor_type, sensor) in sensor_map {
              // Only convert Battery backwards. Other sensors weren't really built for v3 and we
              // never recommended using them or implemented much for them.
              if *sensor_type == InputType::Battery
                && sensor.input_commands().contains(&InputCommandType::Read)
              {
                sensor_vec.push(SensorDeviceMessageAttributesV3 {
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
