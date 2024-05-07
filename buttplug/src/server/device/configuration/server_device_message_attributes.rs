//! A collection of legacy device definitions for the server portion of Buttplug. All structs in
//! this module can be considered deprecated, and will be removed as we move toward Buttplug v4.

use std::{mem, ops::RangeInclusive};

use getset::{Getters, MutGetters, Setters};

use crate::core::message::{
  ActuatorType, ButtplugActuatorFeatureMessageType, ButtplugDeviceMessageType, ButtplugSensorFeatureMessageType, ClientDeviceMessageAttributes, ClientDeviceMessageAttributesBuilder, ClientGenericDeviceMessageAttributes, DeviceFeature, Endpoint, NullDeviceMessageAttributes, RawDeviceMessageAttributes, SensorDeviceMessageAttributes, SensorType
};

use super::UserDeviceDefinition;

/// Device attribute storage and handling
///
/// ProtocolDeviceAttributes represent information about a device in relation to its protocol. This
/// includes the device name, its identifier (assuming it has one), its user created display name
/// (if it has one), and its message attributes.
///
/// Device attributes can exist in 3 different forms for a protocol, as denoted by the
/// [ProtocolAttributesIdentifier].
///
/// - Default: The basis for all message attributes for a protocol. Used when a protocol supports
///   many different devices, all with at least one or more similar features. For instances, we can
///   assume all Lovense devices have a single vibrator with a common power level count, so the
///   Default identifier instance of the ProtocolDeviceAttributes for Lovense will have a
///   message_attributes with VibrateCmd (assuming 1 vibration motor, as all Lovense devices have at
///   least one motor) available.
/// - Identifier: Specifies a specific device for a protocol, which may have its own attributes.
///   Continuing with the Lovense Example, we know a Edge will have 2 motors. We can set the
///   specific Identifier version of the ProtocolDeviceAttributes to have a VibrateCmd
///   message_attributes entry which will override the Default identifier version.
/// - User Configuration: Users may set configurations specific to their setup, like reducing the
///   maximum power available on a device to a certain level. User configurations override the
///   previous Identifier and Default configurations.
///
///  This type of tree/list encoding preserves the structure of configuration, which allows for
///  easier debugging, as well as the ability to serialize the structure back down to files.
#[derive(Debug, Clone, Getters, Setters, MutGetters)]
#[getset(get = "pub")]
pub struct ProtocolDeviceAttributes {
  /// Given name of the device this instance represents.
  name: String,
  /// User configured name of the device this instance represents, assuming one exists.
  display_name: Option<String>,
  /// Message attributes for this device instance.
  message_attributes: ServerDeviceMessageAttributes,
}

impl From<UserDeviceDefinition> for ProtocolDeviceAttributes {
  fn from(mut value: UserDeviceDefinition) -> Self {
    Self {
      name: { mem::take(value.name_mut()) },
      display_name: value.user_config_mut().display_name().clone(),
      message_attributes: { mem::take(value.features_mut()).into() },
    }
  }
}

impl ProtocolDeviceAttributes {
  /// Create a new instance
  pub fn new(
    name: &str,
    display_name: &Option<String>,
    message_attributes: &ServerDeviceMessageAttributes,
  ) -> Self {
    Self {
      name: name.to_owned(),
      display_name: display_name.clone(),
      message_attributes: message_attributes.clone(),
    }
  }

  /// Check if a type of device message is supported by this instance.
  pub fn allows_message(&self, message_type: &ButtplugDeviceMessageType) -> bool {
    self.message_attributes.message_allowed(message_type)
  }

  /// Add raw message support to the attributes of this instance. Requires a list of all endpoints a
  /// device supports.
  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    self.message_attributes.add_raw_messages(endpoints);
  }
}

// Unlike other message components, MessageAttributes is always turned on for
// serialization, because it's used by device configuration files also.
#[derive(
  Clone, Debug, Default, PartialEq, Eq, Getters, MutGetters, Setters,
)]
pub struct ServerDeviceMessageAttributes {
  // Generic commands
  #[getset(get = "pub", get_mut = "pub(super)")]
  scalar_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  rotate_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,
  #[getset(get = "pub", get_mut = "pub(super)")]
  linear_cmd: Option<Vec<ServerGenericDeviceMessageAttributes>>,

  // Sensor Messages
  #[getset(get = "pub")]
  sensor_read_cmd: Option<Vec<SensorDeviceMessageAttributes>>,
  #[getset(get = "pub")]
  sensor_subscribe_cmd: Option<Vec<SensorDeviceMessageAttributes>>,

  // StopDeviceCmd always exists
  #[getset(get = "pub")]
  stop_device_cmd: NullDeviceMessageAttributes,

  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  raw_read_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  raw_write_cmd: Option<RawDeviceMessageAttributes>,
  // Raw commands are only added post-serialization
  #[getset(get = "pub")]
  raw_subscribe_cmd: Option<RawDeviceMessageAttributes>,

  // Needed to load from config for fallback, but unused here.
  #[getset(get = "pub")]
  fleshlight_launch_fw12_cmd: Option<NullDeviceMessageAttributes>,
  #[getset(get = "pub")]
  vorze_a10_cyclone_cmd: Option<NullDeviceMessageAttributes>,
}

impl From<Vec<DeviceFeature>> for ServerDeviceMessageAttributes {
  fn from(features: Vec<DeviceFeature>) -> Self {
    let actuator_filter = |message_type| {
      let attrs: Vec<ServerGenericDeviceMessageAttributes> = features
        .iter()
        .filter(|x| {
          if let Some(actuator) = x.actuator() {
            actuator.messages().contains(message_type)
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
      let attrs: Vec<SensorDeviceMessageAttributes> = features
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
    let raw_attrs = if let Some(raw_feature) = features.iter().find(|x| x.raw().is_some()) {
      Some(RawDeviceMessageAttributes::new(
        raw_feature.raw().as_ref().unwrap().endpoints(),
      ))
    } else {
      None
    };

    Self {
      scalar_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::ScalarCmd),
      rotate_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::RotateCmd),
      linear_cmd: actuator_filter(&ButtplugActuatorFeatureMessageType::LinearCmd),
      sensor_read_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorReadCmd),
      sensor_subscribe_cmd: sensor_filter(&ButtplugSensorFeatureMessageType::SensorSubscribeCmd),
      raw_read_cmd: raw_attrs.clone(),
      raw_subscribe_cmd: raw_attrs.clone(),
      raw_write_cmd: raw_attrs.clone(),
      ..Default::default()
    }
  }
}

impl ServerDeviceMessageAttributes {
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

  pub fn merge(&self, child: &ServerDeviceMessageAttributes) -> ServerDeviceMessageAttributes {
    Self {
      rotate_cmd: child
        .rotate_cmd()
        .clone()
        .or_else(|| self.rotate_cmd().clone()),
      linear_cmd: child
        .linear_cmd()
        .clone()
        .or_else(|| self.linear_cmd().clone()),
      scalar_cmd: child
        .scalar_cmd()
        .clone()
        .or_else(|| self.scalar_cmd().clone()),
      sensor_read_cmd: child
        .sensor_read_cmd()
        .clone()
        .or_else(|| self.sensor_read_cmd().clone()),
      sensor_subscribe_cmd: child
        .sensor_subscribe_cmd()
        .clone()
        .or_else(|| self.sensor_subscribe_cmd().clone()),
      stop_device_cmd: NullDeviceMessageAttributes::default(),
      raw_read_cmd: child
        .raw_read_cmd()
        .clone()
        .or_else(|| self.raw_read_cmd().clone()),
      raw_write_cmd: child
        .raw_write_cmd()
        .clone()
        .or_else(|| self.raw_write_cmd().clone()),
      raw_subscribe_cmd: child
        .raw_subscribe_cmd()
        .clone()
        .or_else(|| self.raw_subscribe_cmd().clone()),
      fleshlight_launch_fw12_cmd: child
        .fleshlight_launch_fw12_cmd()
        .clone()
        .or_else(|| self.fleshlight_launch_fw12_cmd().clone()),
      vorze_a10_cyclone_cmd: child
        .vorze_a10_cyclone_cmd()
        .clone()
        .or_else(|| self.vorze_a10_cyclone_cmd().clone()),
    }
  }

  pub fn add_raw_messages(&mut self, endpoints: &[Endpoint]) {
    let raw_attrs = RawDeviceMessageAttributes::new(endpoints);
    self.raw_read_cmd = Some(raw_attrs.clone());
    self.raw_write_cmd = Some(raw_attrs.clone());
    self.raw_subscribe_cmd = Some(raw_attrs);
  }
}

impl From<ServerDeviceMessageAttributes> for ClientDeviceMessageAttributes {
  fn from(attrs: ServerDeviceMessageAttributes) -> Self {
    let mut builder = ClientDeviceMessageAttributesBuilder::default();
    if let Some(scalar_cmd) = attrs.scalar_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        scalar_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.scalar_cmd(&commands);
    }
    if let Some(rotate_cmd) = attrs.rotate_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        rotate_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.rotate_cmd(&commands);
    }
    if let Some(linear_cmd) = attrs.linear_cmd {
      let commands: Vec<ClientGenericDeviceMessageAttributes> =
        linear_cmd.iter().cloned().map(|x| x.into()).collect();
      builder.linear_cmd(&commands);
    }
    if let Some(sensor_read_cmd) = attrs.sensor_read_cmd {
      builder.sensor_read_cmd(&sensor_read_cmd);
    }
    if let Some(sensor_subscribe_cmd) = attrs.sensor_subscribe_cmd {
      builder.sensor_subscribe_cmd(&sensor_subscribe_cmd);
    }
    if let Some(raw_read_cmd) = attrs.raw_read_cmd {
      builder.raw_read_cmd(raw_read_cmd.endpoints());
    }
    if let Some(raw_write_cmd) = attrs.raw_write_cmd {
      builder.raw_write_cmd(raw_write_cmd.endpoints());
    }
    if let Some(raw_subscribe_cmd) = attrs.raw_subscribe_cmd {
      builder.raw_subscribe_cmd(raw_subscribe_cmd.endpoints());
    }
    builder.finish()
  }
}

#[derive(Default)]
pub struct ServerDeviceMessageAttributesBuilder {
  attrs: ServerDeviceMessageAttributes,
}

impl ServerDeviceMessageAttributesBuilder {
  pub fn scalar_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.scalar_cmd = Some(attrs.to_vec());
    self
  }

  pub fn rotate_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
    self.attrs.rotate_cmd = Some(attrs.to_vec());
    self
  }

  pub fn linear_cmd(&mut self, attrs: &[ServerGenericDeviceMessageAttributes]) -> &Self {
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

  pub fn finish(&self) -> ServerDeviceMessageAttributes {
    self.attrs.clone()
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Getters, Setters)]
pub struct ServerGenericDeviceMessageAttributes {
  #[getset(get = "pub")]
  feature_descriptor: String,
  #[getset(get = "pub")]
  actuator_type: ActuatorType,
  #[getset(get = "pub", set = "pub")]
  step_range: RangeInclusive<u32>,
  #[getset(get = "pub", set = "pub")]
  step_limit: RangeInclusive<u32>,
}

impl From<ServerGenericDeviceMessageAttributes> for ClientGenericDeviceMessageAttributes {
  fn from(attrs: ServerGenericDeviceMessageAttributes) -> Self {
    ClientGenericDeviceMessageAttributes::new(
      &attrs.feature_descriptor,
      attrs.step_count(),
      attrs.actuator_type,
    )
  }
}

impl TryFrom<DeviceFeature> for ServerGenericDeviceMessageAttributes {
  type Error = String;
  fn try_from(value: DeviceFeature) -> Result<Self, Self::Error> {
    if let Some(actuator) = value.actuator() {
      let actuator_type = (*value.feature_type()).try_into()?;
      let attrs = Self {
        feature_descriptor: value.description().to_owned(),
        actuator_type,
        step_range: actuator.step_range().clone(),
        step_limit: actuator.step_limit().clone(),
      };
      Ok(attrs)
    } else {
      Err(format!(
        "Cannot produce a GenericDeviceMessageAttribute from a feature with no actuator member"
      ))
    }
  }
}

impl ServerGenericDeviceMessageAttributes {
  pub fn step_count(&self) -> u32 {
    self.step_limit.end() - self.step_limit.start()
  }
}

#[cfg(test)]
mod test {
  use std::collections::HashSet;

use crate::core::message::DeviceFeatureActuator;

use super::*;

  #[test]
  pub fn test_step_count_calculation() {
    let device_feature = DeviceFeature::new(
      "test", 
      crate::core::message::FeatureType::Vibrate, 
      &Some(DeviceFeatureActuator::new(&RangeInclusive::new(0, 10), &RangeInclusive::new(0, 10), &HashSet::from([ButtplugActuatorFeatureMessageType::ScalarCmd]))), 
      &None);

    let vibrate_attributes: ServerGenericDeviceMessageAttributes = device_feature.try_into().unwrap();
    assert_eq!(vibrate_attributes.step_count(), 10);

    let device_feature_2 = DeviceFeature::new(
      "test", 
      crate::core::message::FeatureType::Vibrate, 
      &Some(DeviceFeatureActuator::new(&RangeInclusive::new(0, 10), &RangeInclusive::new(3, 7), &HashSet::from([ButtplugActuatorFeatureMessageType::ScalarCmd]))), 
      &None);
    let vibrate_attributes_2: ServerGenericDeviceMessageAttributes = device_feature_2.try_into().unwrap();
    assert_eq!(vibrate_attributes_2.step_count(), 4);
  }
}
