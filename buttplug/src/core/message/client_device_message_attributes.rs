// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::core::{
  errors::ButtplugDeviceError,
  message::{ButtplugDeviceMessageType, Endpoint},
};
use getset::{Getters, MutGetters, Setters};
use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::ops::RangeInclusive;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorType {
  Unknown,
  Vibrate,
  // Single Direction Rotation Speed
  Rotate,
  Oscillate,
  Constrict,
  Inflate,
  // For instances where we specify a position to move to ASAP. Usually servos, probably for the
  // OSR-2/SR-6.
  Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum SensorType {
  Unknown,
  Battery,
  RSSI,
  Button,
  Pressure,
  // Temperature,
  // Accelerometer,
  // Gyro,
}

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
pub struct ClientDeviceMessageAttributes {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "ScalarCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  scalar_cmd: Option<Vec<ClientGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<Vec<ClientGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<Vec<ClientGenericDeviceMessageAttributes>>,

  // Sensor Messages
  #[getset(get = "pub")]
  #[serde(rename = "SensorReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributes>>,
  #[getset(get = "pub")]
  #[serde(rename = "SensorSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributes>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  #[serde(skip_deserializing)]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_read_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_write_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_deserializing)]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributes>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip_serializing)]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip_serializing)]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl ClientDeviceMessageAttributes {
  pub fn raw_unsubscribe_cmd(&self) -> &Option<RawDeviceMessageAttributes> {
    self.raw_subscribe_cmd()
  }

  pub fn message_allowed(&self, message_type: &ButtplugDeviceMessageType) -> bool {
    match message_type {
      ButtplugDeviceMessageType::ScalarCmd => self.scalar_cmd.is_some(),
      // VibrateCmd and SingleMotorVibrateCmd will derive from Scalars, so errors will be thrown in
      // the scalar parser if the actuator isn't correct.
      ButtplugDeviceMessageType::VibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SingleMotorVibrateCmd => self.scalar_cmd.is_some(),
      ButtplugDeviceMessageType::SensorReadCmd => self.sensor_read_cmd.is_some(),
      ButtplugDeviceMessageType::SensorSubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::SensorUnsubscribeCmd => self.sensor_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::LinearCmd => self.linear_cmd.is_some(),
      ButtplugDeviceMessageType::RotateCmd => self.rotate_cmd.is_some(),
      ButtplugDeviceMessageType::BatteryLevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::Battery)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd => {
        self.fleshlight_launch_fw12_cmd.is_some()
      }
      ButtplugDeviceMessageType::RSSILevelCmd => {
        if let Some(sensor_info) = &self.sensor_read_cmd {
          sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::RSSI)
        } else {
          false
        }
      }
      ButtplugDeviceMessageType::RawReadCmd => self.raw_read_cmd.is_some(),
      ButtplugDeviceMessageType::RawSubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawUnsubscribeCmd => self.raw_subscribe_cmd.is_some(),
      ButtplugDeviceMessageType::RawWriteCmd => self.raw_write_cmd.is_some(),
      ButtplugDeviceMessageType::VorzeA10CycloneCmd => self.vorze_a10_cyclone_cmd.is_some(),
      ButtplugDeviceMessageType::StopDeviceCmd => true,
      ButtplugDeviceMessageType::KiirooCmd => false,
      ButtplugDeviceMessageType::LovenseCmd => false,
    }
  }

  pub fn finalize(&mut self) {
    if let Some(scalar_attrs) = &mut self.scalar_cmd {
      for (i, attr) in scalar_attrs.into_iter().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_read_attrs) = &mut self.sensor_read_cmd {
      for (i, attr) in sensor_read_attrs.into_iter().enumerate() {
        attr.index = i as u32;
      }
    }
    if let Some(sensor_subscribe_attrs) = &mut self.sensor_subscribe_cmd {
      for (i, attr) in sensor_subscribe_attrs.into_iter().enumerate() {
        attr.index = i as u32;
      }
    }
  }
}

#[derive(Default)]
pub struct ClientDeviceMessageAttributesBuilder {
  attrs: ClientDeviceMessageAttributes,
}

impl ClientDeviceMessageAttributesBuilder {
  pub fn scalar_cmd(&mut self, attrs: &[ClientGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.scalar_cmd = Some(attrs.to_vec());
    self
  }

  pub fn rotate_cmd(&mut self, attrs: &[ClientGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.rotate_cmd = Some(attrs.to_vec());
    self
  }

  pub fn linear_cmd(&mut self, attrs: &[ClientGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.linear_cmd = Some(attrs.to_vec());
    self
  }

  pub fn sensor_read_cmd(&mut self, attrs: &[SensorDeviceMessageAttributes]) -> &Self {
    self.attrs.sensor_read_cmd = Some(attrs.to_vec());
    self
  }

  pub fn sensor_subscribe_cmd(&mut self, attrs: &[SensorDeviceMessageAttributes]) -> &Self {
    self.attrs.sensor_subscribe_cmd = Some(attrs.to_vec());
    self
  }

  pub fn raw_read_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_read_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_write_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_write_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn raw_subscribe_cmd(&mut self, endpoints: &[Endpoint]) -> &Self {
    self.attrs.raw_subscribe_cmd = Some(RawDeviceMessageAttributes::new(endpoints));
    self
  }

  pub fn finish(&mut self) -> ClientDeviceMessageAttributes {
    self.attrs.finalize();
    self.attrs.clone()
  }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NullDeviceMessageAttributes {}

fn unspecified_feature() -> String {
  "N/A".to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientGenericDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  #[serde(default = "unspecified_feature")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "ActuatorType")]
  actuator_type: ActuatorType,
  #[serde(rename = "StepCount")]
  #[getset(get = "pub")]
  step_count: u32,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  index: u32,
}

impl ClientGenericDeviceMessageAttributes {
  pub fn new(feature_descriptor: &str, step_count: u32, actuator_type: ActuatorType) -> Self {
    Self {
      feature_descriptor: feature_descriptor.to_owned(),
      actuator_type,
      step_count,
      index: 0,
    }
  }

  // This is created out of already verified server device message attributes, so we'll assume it's
  // fine.
  pub fn is_valid(&self, _: &ButtplugDeviceMessageType) -> Result<(), ButtplugDeviceError> {
    Ok(())
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, Getters, Setters)]
pub struct RawDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "Endpoints")]
  endpoints: Vec<Endpoint>,
}

impl RawDeviceMessageAttributes {
  pub fn new(endpoints: &[Endpoint]) -> Self {
    Self {
      endpoints: endpoints.to_vec(),
    }
  }
}

fn range_sequence_serialize<S>(
  range_vec: &Vec<RangeInclusive<u32>>,
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
pub struct SensorDeviceMessageAttributes {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureDescriptor")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  #[serde(rename = "SensorType")]
  sensor_type: SensorType,
  #[getset(get = "pub")]
  #[serde(rename = "SensorRange", serialize_with = "range_sequence_serialize")]
  sensor_range: Vec<RangeInclusive<u32>>,
  // TODO This needs to actually be part of the device info relayed to the client in spec v4.
  #[getset(get = "pub")]
  #[serde(skip, default)]
  index: u32,
}

/*
impl SensorDeviceMessageAttributes {
  pub fn new(feature_descriptor: &str, sensor_type: SensorType) -> Self {
    Self { feature_descriptor: feature_descriptor.to_owned(), sensor_type }
  }
}
 */

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientDeviceMessageAttributesV2 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<GenericDeviceMessageAttributesV2>,
  #[getset(get = "pub")]
  #[serde(rename = "BatteryLevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  battery_level_cmd: Option<NullDeviceMessageAttributes>,

  // RSSILevel is added post-serialization (only for bluetooth devices)
  #[getset(get = "pub")]
  #[serde(rename = "RSSILevelCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rssi_level_cmd: Option<NullDeviceMessageAttributes>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  #[serde(rename = "StopDeviceCmd")]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(rename = "RawReadCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_read_cmd: Option<RawDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "RawWriteCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_write_cmd: Option<RawDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "RawSubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "RawUnsubscribeCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  raw_unsubscribe_cmd: Option<RawDeviceMessageAttributes>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  #[serde(rename = "FleshlightLaunchFW12Cmd")]
  #[serde(skip)]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(rename = "VorzeA10CycloneCmd")]
  #[serde(skip)]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl From<ClientDeviceMessageAttributes> for ClientDeviceMessageAttributesV2 {
  fn from(other: ClientDeviceMessageAttributes) -> Self {
    Self {
      vibrate_cmd: other
        .scalar_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::vibrate_cmd_from_scalar_cmd(x))
        .filter(|x| x.feature_count != 0),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV2::from(x.clone())),
      battery_level_cmd: {
        if let Some(sensor_info) = &other.sensor_read_cmd {
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::Battery)
          {
            Some(NullDeviceMessageAttributes::default())
          } else {
            None
          }
        } else {
          None
        }
      },
      rssi_level_cmd: {
        if let Some(sensor_info) = &other.sensor_read_cmd {
          if sensor_info
            .iter()
            .any(|x| *x.sensor_type() == SensorType::RSSI)
          {
            Some(NullDeviceMessageAttributes::default())
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributesV2 {
  #[getset(get = "pub")]
  #[serde(rename = "FeatureCount")]
  feature_count: u32,
  #[getset(get = "pub")]
  #[serde(rename = "StepCount")]
  step_count: Vec<u32>,
}

impl GenericDeviceMessageAttributesV2 {
  pub fn vibrate_cmd_from_scalar_cmd(
    attributes_vec: &[ClientGenericDeviceMessageAttributes],
  ) -> Self {
    let mut feature_count = 0u32;
    let mut step_count = vec![];
    for attr in attributes_vec {
      if *attr.actuator_type() == ActuatorType::Vibrate {
        feature_count += 1;
        step_count.push(*attr.step_count());
      }
    }
    Self {
      feature_count,
      step_count,
    }
  }
}

impl From<Vec<ClientGenericDeviceMessageAttributes>> for GenericDeviceMessageAttributesV2 {
  fn from(attributes_vec: Vec<ClientGenericDeviceMessageAttributes>) -> Self {
    Self {
      feature_count: attributes_vec.len() as u32,
      step_count: attributes_vec.iter().map(|x| *x.step_count()).collect(),
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct ClientDeviceMessageAttributesV1 {
  // Generic commands
  #[getset(get = "pub")]
  #[serde(rename = "VibrateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vibrate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "RotateCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  rotate_cmd: Option<GenericDeviceMessageAttributesV1>,
  #[getset(get = "pub")]
  #[serde(rename = "LinearCmd")]
  #[serde(skip_serializing_if = "Option::is_none")]
  linear_cmd: Option<GenericDeviceMessageAttributesV1>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Obsolete commands are only added post-serialization
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  single_motor_vibrate_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  #[serde(skip_serializing_if = "Option::is_none")]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl From<ClientDeviceMessageAttributesV2> for ClientDeviceMessageAttributesV1 {
  fn from(other: ClientDeviceMessageAttributesV2) -> Self {
    Self {
      vibrate_cmd: other
        .vibrate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      rotate_cmd: other
        .rotate_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      linear_cmd: other
        .linear_cmd()
        .as_ref()
        .map(|x| GenericDeviceMessageAttributesV1::from(x.clone())),
      stop_device_cmd: other.stop_device_cmd().clone(),
      fleshlight_launch_fw12_cmd: other.fleshlight_launch_fw12_cmd().clone(),
      vorze_a10_cyclone_cmd: other.vorze_a10_cyclone_cmd().clone(),
      single_motor_vibrate_cmd: if other.vibrate_cmd().is_some() {
        Some(NullDeviceMessageAttributes::default())
      } else {
        None
      },
    }
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Getters, Setters)]
pub struct GenericDeviceMessageAttributesV1 {
  #[serde(rename = "FeatureCount")]
  feature_count: u32,
}

impl From<GenericDeviceMessageAttributesV2> for GenericDeviceMessageAttributesV1 {
  fn from(attributes: GenericDeviceMessageAttributesV2) -> Self {
    Self {
      feature_count: *attributes.feature_count(),
    }
  }
}
