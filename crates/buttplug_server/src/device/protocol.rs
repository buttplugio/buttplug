// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Implementations of communication protocols for hardware supported by Buttplug

use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{InputData, InputReadingV4, InputType, OutputCommand},
};
use buttplug_server_device_config::{
  Endpoint,
  DeviceDefinition,
  ProtocolCommunicationSpecifier,
  UserDeviceIdentifier,
};
use dashmap::DashMap;

use crate::{
  device::{hardware::{Hardware, HardwareCommand, HardwareReadCmd}, protocol_impl::get_default_protocol_map},
  message::{
    checked_output_cmd::CheckedOutputCmdV4,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
    ButtplugServerDeviceMessage,
  },
};
use async_trait::async_trait;
use futures::{
  future::{self, BoxFuture, FutureExt},
  StreamExt,
};
use std::{collections::HashMap, sync::Arc};
use std::{pin::Pin, time::Duration};
use uuid::Uuid;
use super::hardware::HardwareWriteCmd;

/// Strategy for situations where hardware needs to get updates every so often in order to keep
/// things alive. Currently this applies to iOS backgrounding with bluetooth devices, as well as
/// some protocols like Satisfyer and Mysteryvibe that need constant command refreshing, but since
/// we never know which of our hundreds of supported devices someone might connect, we need context
/// as to which keepalive strategy to use.
///
/// When choosing a keepalive strategy for a protocol:
///
/// - If the protocol has a command that essentially does nothing to the actuators, set up
///   RepeatPacketStrategy to use that. This is useful for devices that have info commands (like
///   Lovense), ping commands (like The Handy), sensor commands that aren't yet subscribed to output
///   notifications, etc...
/// - If a protocol needs specific timing or keepalives, regardless of the OS/hardware manager being
///   used, like Satisfyer or Mysteryvibe, use RepeatLastPacketStrategyWithTiming.
/// - For many devices with only scalar actuators, RepeatLastPacketStrategy should work. You just
///   need to make sure the protocol doesn't have a packet counter or something else that will trip
///   if the same packet is replayed multiple times.
#[derive(Debug)]
pub enum ProtocolKeepaliveStrategy {
  /// Repeat a specific packet, such as a ping or a no-op. Only do this when the hardware manager
  /// requires it (currently only iOS bluetooth during backgrounding).
  HardwareRequiredRepeatPacketStrategy(HardwareWriteCmd),
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Uses
  /// a default timing, suitable for most protocols that don't need constant device updates outside
  /// of OS requirements. Only do this when the hardware manager requires it (currently only iOS
  /// bluetooth during backgrounding).
  HardwareRequiredRepeatLastPacketStrategy,
  /// Repeat whatever the last packet sent was, and send Stop commands until first packet sent. Do
  /// this regardless of whether or not the hardware manager requires it. Useful for hardware that
  /// requires keepalives, like Satisfyer, Mysteryvibe, Leten, etc...
  RepeatLastPacketStrategyWithTiming(Duration),
}

pub trait ProtocolIdentifierFactory: Send + Sync {
  fn identifier(&self) -> &str;
  fn create(&self) -> Box<dyn ProtocolIdentifier>;
}

pub enum ProtocolValueCommandPrefilterStrategy {
  /// Drop repeated ValueCmd/ValueWithParameterCmd messages
  DropRepeats,
  /// No filter, send all value messages for processing
  None,
}

fn print_type_of<T>(_: &T) -> &'static str {
  std::any::type_name::<T>()
}

pub struct ProtocolSpecializer {
  specifiers: Vec<ProtocolCommunicationSpecifier>,
  identifier: Box<dyn ProtocolIdentifier>,
}

impl ProtocolSpecializer {
  pub fn new(
    specifiers: Vec<ProtocolCommunicationSpecifier>,
    identifier: Box<dyn ProtocolIdentifier>,
  ) -> Self {
    Self {
      specifiers,
      identifier,
    }
  }

  pub fn specifiers(&self) -> &Vec<ProtocolCommunicationSpecifier> {
    &self.specifiers
  }

  pub fn identify(self) -> Box<dyn ProtocolIdentifier> {
    self.identifier
  }
}

#[async_trait]
pub trait ProtocolIdentifier: Sync + Send {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    specifier: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError>;
}

#[async_trait]
pub trait ProtocolInitializer: Sync + Send {
  async fn initialize(
    &mut self,
    hardware: Arc<Hardware>,
    device_definition: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError>;
}

pub struct GenericProtocolIdentifier {
  handler: Option<Arc<dyn ProtocolHandler>>,
  protocol_identifier: String,
}

impl GenericProtocolIdentifier {
  pub fn new(handler: Arc<dyn ProtocolHandler>, protocol_identifier: &str) -> Self {
    Self {
      handler: Some(handler),
      protocol_identifier: protocol_identifier.to_owned(),
    }
  }
}

#[async_trait]
impl ProtocolIdentifier for GenericProtocolIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let device_identifier = UserDeviceIdentifier::new(
      hardware.address(),
      &self.protocol_identifier,
      &Some(hardware.name().to_owned()),
    );
    Ok((
      device_identifier,
      Box::new(GenericProtocolInitializer::new(
        self.handler.take().unwrap(),
      )),
    ))
  }
}

pub struct GenericProtocolInitializer {
  handler: Option<Arc<dyn ProtocolHandler>>,
}

impl GenericProtocolInitializer {
  pub fn new(handler: Arc<dyn ProtocolHandler>) -> Self {
    Self {
      handler: Some(handler),
    }
  }
}

#[async_trait]
impl ProtocolInitializer for GenericProtocolInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    _: &DeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    Ok(self.handler.take().unwrap())
  }
}

pub trait ProtocolHandler: Sync + Send {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  fn handle_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnionV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented(print_type_of(&message))
  }

  // Allow here since this changes between debug/release
  #[allow(unused_variables)]
  fn command_unimplemented(
    &self,
    command: &str,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    #[cfg(debug_assertions)]
    unimplemented!("Command not implemented for this protocol");
    #[cfg(not(debug_assertions))]
    Err(ButtplugDeviceError::UnhandledCommand(format!(
      "Command not implemented for this protocol: {}",
      command
    )))
  }

  // The default scalar handler assumes that most devices require discrete commands per feature. If
  // a protocol has commands that combine multiple features, either with matched or unmatched
  // actuators, they should just implement their own version of this method.
  fn handle_output_cmd(
    &self,
    cmd: &CheckedOutputCmdV4,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let output_command = cmd.output_command();
    match output_command {
      OutputCommand::Constrict(x) => {
        self.handle_output_constrict_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Spray(x) => {
        self.handle_output_spray_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Oscillate(x) => {
        self.handle_output_oscillate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Rotate(x) => {
        self.handle_output_rotate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Vibrate(x) => {
        self.handle_output_vibrate_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Position(x) => {
        self.handle_output_position_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Heater(x) => {
        self.handle_output_heater_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::Led(x) => {
        self.handle_output_led_cmd(cmd.feature_index(), cmd.feature_id(), x.value())
      }
      OutputCommand::PositionWithDuration(x) => self.handle_position_with_duration_cmd(
        cmd.feature_index(),
        cmd.feature_id(),
        x.position(),
        x.duration(),
      ),
      OutputCommand::RotateWithDirection(x) => self.handle_rotation_with_direction_cmd(
        cmd.feature_index(),
        cmd.feature_id(),
        x.speed(),
        x.clockwise(),
      ),
    }
  }

  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Vibrate Actuator)")
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Rotate Actuator)")
  }

  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Oscillate Actuator)")
  }

  fn handle_output_spray_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Spray Actuator)")
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Constrict Actuator)")
  }

  fn handle_output_heater_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Heater Actuator)")
  }

  fn handle_output_led_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Led Actuator)")
  }

  fn handle_output_position_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Position Actuator)")
  }

  fn handle_position_with_duration_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _position: u32,
    _duration: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Position w/ Duration Actuator)")
  }

  fn handle_rotation_with_direction_cmd(
    &self,
    _feature_index: u32,
    _feature_id: Uuid,
    _speed: u32,
    _clockwise: bool,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    self.command_unimplemented("OutputCmd (Rotation w/ Direction Actuator)")
  }

  fn handle_input_subscribe_cmd(
    &self,
    _device_index: u32,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: InputCmd (Subscribe)".to_string(),
    )))
    .boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
    _sensor_type: InputType,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: InputCmd (Unsubscribe)".to_string(),
    )))
    .boxed()
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    match sensor_type {
      InputType::Battery => {
        self.handle_battery_level_cmd(device_index, device, feature_index, feature_id)
      }
      _ => future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this protocol: InputCmd (Read)".to_string(),
      )))
      .boxed(),
    }
  }

  // Handle Battery Level returns a SensorReading, as we'll always need to do a sensor index
  // conversion on it.
  fn handle_battery_level_cmd(
    &self,
    device_index: u32,
    device: Arc<Hardware>,
    feature_index: u32,
    feature_id: Uuid,
  ) -> BoxFuture<Result<InputReadingV4, ButtplugDeviceError>> {
    // If we have a standardized BLE Battery endpoint, handle that above the
    // protocol, as it'll always be the same.
    if device.endpoints().contains(&Endpoint::RxBLEBattery) {
      debug!("Trying to get battery reading.");
      let msg = HardwareReadCmd::new(feature_id, Endpoint::RxBLEBattery, 1, 0);
      let fut = device.read_value(&msg);
      async move {
        let hw_msg = fut.await?;
        let battery_level = hw_msg.data()[0] as i32;
        let battery_reading = InputReadingV4::new(
          device_index,
          feature_index,
          buttplug_core::message::InputTypeData::Battery(InputData::new(battery_level as u8))
        );
        debug!("Got battery reading: {}", battery_level);
        Ok(battery_reading)
      }
      .boxed()
    } else {
      future::ready(Err(ButtplugDeviceError::UnhandledCommand(
        "Command not implemented for this protocol: SensorReadCmd".to_string(),
      )))
      .boxed()
    }
  }

  fn handle_rssi_level_cmd(
    &self,
    _device: Arc<Hardware>,
    _feature_index: u32,
    _feature_id: Uuid,
  ) -> BoxFuture<Result<(), ButtplugDeviceError>> {
    future::ready(Err(ButtplugDeviceError::UnhandledCommand(
      "Command not implemented for this protocol: SensorReadCmd".to_string(),
    )))
    .boxed()
  }

  fn event_stream(
    &self,
  ) -> Pin<Box<dyn tokio_stream::Stream<Item = ButtplugServerDeviceMessage> + Send>> {
    tokio_stream::empty().boxed()
  }
}

#[macro_export]
macro_rules! generic_protocol_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      pub mod setup {
        use std::sync::Arc;
        use $crate::device::protocol::{
          GenericProtocolIdentifier, ProtocolIdentifier, ProtocolIdentifierFactory,
        };
        #[derive(Default)]
        pub struct [< $protocol_name IdentifierFactory >] {}

        impl ProtocolIdentifierFactory for  [< $protocol_name IdentifierFactory >] {
          fn identifier(&self) -> &str {
            $protocol_identifier
          }

          fn create(&self) -> Box<dyn ProtocolIdentifier> {
            Box::new(GenericProtocolIdentifier::new(
              Arc::new(super::$protocol_name::default()),
              self.identifier(),
            ))
          }
        }
      }
    }
  };
}

#[macro_export]
macro_rules! generic_protocol_initializer_setup {
  ( $protocol_name:ident, $protocol_identifier:tt) => {
    paste::paste! {
      pub mod setup {
        use $crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
        #[derive(Default)]
        pub struct [< $protocol_name IdentifierFactory >] {}

        impl ProtocolIdentifierFactory for [< $protocol_name IdentifierFactory >] {
          fn identifier(&self) -> &str {
            $protocol_identifier
          }

          fn create(&self) -> Box<dyn ProtocolIdentifier> {
            Box::new(super::[< $protocol_name Identifier >]::default())
          }
        }
      }

      #[derive(Default)]
      pub struct [< $protocol_name Identifier >] {}

      #[async_trait]
      impl ProtocolIdentifier for [< $protocol_name Identifier >] {
        async fn identify(
          &mut self,
          hardware: Arc<Hardware>,
          _: ProtocolCommunicationSpecifier,
        ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
          Ok((UserDeviceIdentifier::new(hardware.address(), $protocol_identifier, &Some(hardware.name().to_owned())), Box::new([< $protocol_name Initializer >]::default())))
        }
      }
    }
  };
}

pub use generic_protocol_initializer_setup;
pub use generic_protocol_setup;

pub struct ProtocolManager {
  // Map of protocol names to their respective protocol instance factories
  protocol_map: HashMap<String, Arc<dyn ProtocolIdentifierFactory>>,
}

impl Default for ProtocolManager {
  fn default() -> Self {
    Self {
      protocol_map: get_default_protocol_map()
    }
  }
}

impl ProtocolManager {
  pub fn protocol_specializers(
    &self,
    specifier: &ProtocolCommunicationSpecifier,
    base_communication_specifiers: &HashMap<String, Vec<ProtocolCommunicationSpecifier>>,
    user_communication_specifiers: &DashMap<String, Vec<ProtocolCommunicationSpecifier>>,
  ) -> Vec<ProtocolSpecializer> {
    debug!(
      "Looking for protocol that matches specifier: {:?}",
      specifier
    );
    let mut specializers = vec![];
    let mut update_specializer_map =
      |name: &str, specifiers: &Vec<ProtocolCommunicationSpecifier>| {
        if specifiers.contains(specifier) {
          info!(
            "Found protocol {:?} for user specifier {:?}.",
            name, specifier
          );
          if self.protocol_map.contains_key(name) {
            specializers.push(ProtocolSpecializer::new(
              specifiers.clone(),
              self
                .protocol_map
                .get(name)
                .expect("already checked existence")
                .create(),
            ));
          } else {
            warn!(
              "No protocol implementation for {:?} found for specifier {:?}.",
              name, specifier
            );
          }
        }
      };
    // Loop through both maps, as chaining between DashMap and HashMap gets kinda gross.
    for spec in user_communication_specifiers.iter() {
      update_specializer_map(spec.key(), spec.value());
    }
    for (name, specifiers) in base_communication_specifiers.iter() {
      update_specializer_map(name, specifiers);
    }
    specializers
  }
}


/*
#[cfg(test)]
mod test {
  use super::*;
  use crate::{
    core::message::{OutputType, FeatureType},
    server::message::server_device_feature::{ServerDeviceFeature, ServerDeviceFeatureOutput},
  };
  use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
  };

  fn create_unit_test_dcm() -> DeviceConfigurationManager {
    let mut builder = DeviceConfigurationManagerBuilder::default();
    let specifiers = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new(
      HashSet::from(["LVS-*".to_owned(), "LovenseDummyTestName".to_owned()]),
      vec![],
      HashSet::new(),
      HashMap::new(),
    ));
    let mut feature_actuator = HashMap::new();
    feature_actuator.insert(
      OutputType::Vibrate,
      ServerDeviceFeatureOutput::new(&RangeInclusive::new(0, 20), &RangeInclusive::new(0, 20)),
    );
    builder
      .communication_specifier("lovense", &[specifiers])
      .protocol_features(
        &BaseDeviceIdentifier::new("lovense", &Some("P".to_owned())),
        &BaseDeviceDefinition::new(
          "Lovense Edge",
          &uuid::Uuid::new_v4(),
          &None,
          &vec![
            ServerDeviceFeature::new(
              "Edge Vibration 1",
              &uuid::Uuid::new_v4(),
              &None,
              FeatureType::Vibrate,
              &Some(feature_actuator.clone()),
              &None,
            ),
            ServerDeviceFeature::new(
              "Edge Vibration 2",
              &uuid::Uuid::new_v4(),
              &None,
              FeatureType::Vibrate,
              &Some(feature_actuator.clone()),
              &None,
            ),
          ],
          &None
        ),
      )
      .finish()
      .unwrap()
  }

  #[test]
  fn test_config_equals() {
    let config = create_unit_test_dcm();
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Something",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }

  #[test]
  fn test_config_wildcard_equals() {
    let config = create_unit_test_dcm();
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!config.protocol_specializers(&spec).is_empty());
  }
  /*
  #[test]
  fn test_specific_device_config_creation() {
    let dcm = create_unit_test_dcm(false);
    let spec = ProtocolCommunicationSpecifier::BluetoothLE(BluetoothLESpecifier::new_from_device(
      "LVS-Whatever",
      &HashMap::new(),
      &[],
    ));
    assert!(!dcm.protocol_specializers(&spec).is_empty());
    let config: ProtocolDeviceAttributes = dcm
      .device_definition(
        &UserDeviceIdentifier::new("Whatever", "lovense", &Some("P".to_owned())),
        &[],
      )
      .expect("Should be found")
      .into();
    // Make sure we got the right name
    assert_eq!(config.name(), "Lovense Edge");
    // Make sure we overwrote the default of 1
    assert_eq!(
      config
        .message_attributes()
        .scalar_cmd()
        .as_ref()
        .expect("Test, assuming infallible")
        .get(0)
        .expect("Test, assuming infallible")
        .step_count(),
      20
    );
  }
  */
}
*/
