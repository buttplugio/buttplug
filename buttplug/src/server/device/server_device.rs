// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.



use std::{
  fmt::{self, Debug},
  sync::Arc,
};

use crate::{
  core::{
    errors::{ButtplugError, ButtplugDeviceError},
    messages::{
      self,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessageType,
      DeviceMessageAttributesMap,
      Endpoint,
      RawSubscribeCmd,
    },
    ButtplugResultFuture,
  },
  server::{
    ButtplugServerResultFuture,
    device::{
      hardware::{Hardware, HardwareConnector, HardwareEvent},
      configuration::{ProtocolAttributesType, DeviceConfigurationManager},
      protocol::{ProtocolIdentifier, ProtocolInitializer, ProtocolHandler}
    },
  },
};
use getset::{Getters, Setters, MutGetters};
use serde::{Serialize, Deserialize};
use core::hash::{Hash, Hasher};
use tokio::sync::broadcast;
use futures::future;

use super::{configuration::ProtocolDeviceAttributes, protocol::generic_command_manager::GenericCommandManager};

/// Identifying information for a connected devices
/// 
/// Contains the 3 fields needed to uniquely identify a device in the system.
#[derive(Debug, Eq, PartialEq, Hash, Clone, Getters, Setters, MutGetters, Serialize, Deserialize)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct ServerDeviceIdentifier {
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  attributes_identifier: ProtocolAttributesType
}

impl ServerDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &ProtocolAttributesType) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      attributes_identifier: identifier.clone()
    }
  }
}

pub(super) async fn build_server_device(device_config_manager: Arc<DeviceConfigurationManager>, mut hardware_connector: Box<dyn HardwareConnector>) -> Result<ServerDevice, ButtplugDeviceError> {

  // First off, we need to see if we even have a configuration available for the device we're
  // trying to create. If we don't, exit, because this isn't actually an error. However, if we
  // *do* have a configuration but something goes wrong after this, then it's an error.
  let protocol_specializers = device_config_manager.protocol_specializers(&hardware_connector.specifier()); 

  // If we have no identifiers, then there's nothing to do here. Throw an error.
  if protocol_specializers.is_empty() {
    return Err(ButtplugDeviceError::DeviceConfigurationError("No viable protocols for hardware.".to_owned()));
  }

  // At this point, we know we've got hardware that is waiting to connect, and enough protocol
  // info to actually do something after we connect. So go ahead and connect.
  let mut hardware_specializer = hardware_connector.connect().await?;

  // We can't run these in parallel because we need to only accept one specializer.
  let mut protocol_identifier = None;
  let mut hardware_out = None;
  for protocol_specializer in protocol_specializers {
    if let Ok(specialized_hardware) = hardware_specializer.specialize(protocol_specializer.specifiers()).await {
      protocol_identifier = Some(protocol_specializer.identify());
      hardware_out = Some(specialized_hardware);
      break;
    }
  }

  if protocol_identifier.is_none() {
    return Err(ButtplugDeviceError::DeviceConfigurationError("No protocols with viable communication matches for hardware.".to_owned()));
  }

  let mut protocol_identifier_stage = protocol_identifier.unwrap();
  let hardware = Arc::new(hardware_out.unwrap());

  let (identifier, mut protocol_initializer) = protocol_identifier_stage.identify(hardware.clone()).await?;

  // Now we have an identifier. After this point, if anything fails, consider it a complete
  // connection failure, as identify may have already run commands on the device, and therefore
  // put it in an unknown state if anything fails.
  
  // Check in the DeviceConfigurationManager to make sure we have attributes
  // for this device.
  let attrs = if let Some(attrs) = device_config_manager.protocol_device_attributes(&identifier, &hardware.endpoints()) {
    attrs
  } else {
    return Err(ButtplugDeviceError::DeviceConfigurationError(format!("No protocols with viable protocol attributes for hardware {:?}.", identifier)));
  };

  // If we have attributes, go ahead and initialize, handing us back our hardware instance that
  // is now ready to use with the protocol handler.
  
  // Build the server device and return.
  
  let handler = protocol_initializer.initialize(hardware.clone()).await?;

  // We now have fully initialized hardware, return a server device.
  Ok(ServerDevice::new(
    identifier,
    handler,
    hardware,
    &attrs
  ))
}

pub struct ServerDevice {
  hardware: Arc<Hardware>,
  handler: Box<dyn ProtocolHandler>,
  attributes: ProtocolDeviceAttributes,
  generic_command_manager: GenericCommandManager,
  /// Unique identifier for the device
  identifier: ServerDeviceIdentifier

}
impl Debug for ServerDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugDevice")
      .field("name", &self.name())
      .field("identifier", &self.identifier)
      .finish()
  }
}

impl Hash for ServerDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.identifier.hash(state);
  }
}

impl Eq for ServerDevice {
}

impl PartialEq for ServerDevice {
  fn eq(&self, other: &Self) -> bool {
    self.identifier == *other.identifier()
  }
}

impl ServerDevice {
  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(identifier: ServerDeviceIdentifier, handler: Box<dyn ProtocolHandler>, hardware: Arc<Hardware>, attributes: &ProtocolDeviceAttributes) -> Self {
    Self {
      identifier,
      generic_command_manager: GenericCommandManager::new(attributes),
      handler,
      hardware,
      attributes: attributes.clone()
    }
  }

  /// Returns the device identifier
  pub fn identifier(&self) -> &ServerDeviceIdentifier {
    &self.identifier
  }

  /// Get the user created display name for a device, if one exists.
  pub fn display_name(&self) -> Option<String> {
    self.attributes.display_name()
  }

  /// Get the name of the device as set in the Device Configuration File.
  /// 
  /// This will also append "(Raw Messaged Allowed)" to the device name if raw mode is on, to warn
  /// users that the device is capable of direct communication.
  pub fn name(&self) -> String {
    // Instead of checking for raw messages at the protocol level, add the raw
    // call here, since this is the only way to access devices in the library
    // anyways.
    //
    // Having raw turned on means it'll work for read/write/sub/unsub on any
    // endpoint so just use an arbitrary message here to check.
    if self
      .supports_message(&ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(
        RawSubscribeCmd::new(1, Endpoint::Tx),
      ))
      .is_ok()
    {
      format!("{} (Raw Messages Allowed)", self.attributes.name())
    } else {
      self.attributes.name().to_owned()
    }
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    self.hardware.disconnect()
  }

  /// Retreive the message attributes for the device.
  pub fn message_attributes(&self) -> DeviceMessageAttributesMap {
    self.attributes.message_attributes_map()
  }

  /// Retreive the event stream for the device.
  /// 
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.hardware.event_stream()
  }  
  
  pub fn stop_commands(&self) -> Vec<ButtplugDeviceCommandMessageUnion> {
    self.generic_command_manager.stop_commands()
  }

  pub fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    let check_msg = |msg_type| self
    .attributes
    .allows_message(&msg_type)
    .then(|| ())
    .ok_or(ButtplugDeviceError::MessageNotSupported(msg_type));

    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => check_msg(ButtplugDeviceMessageType::BatteryLevelCmd),
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => check_msg(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => check_msg(ButtplugDeviceMessageType::KiirooCmd),
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => check_msg(ButtplugDeviceMessageType::LinearCmd),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => check_msg(ButtplugDeviceMessageType::RawReadCmd),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => check_msg(ButtplugDeviceMessageType::RawSubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => check_msg(ButtplugDeviceMessageType::RawUnsubscribeCmd),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => check_msg(ButtplugDeviceMessageType::RawWriteCmd),
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => check_msg(ButtplugDeviceMessageType::RotateCmd),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => check_msg(ButtplugDeviceMessageType::RSSILevelCmd),
      ButtplugDeviceCommandMessageUnion::LevelCmd(_) => check_msg(ButtplugDeviceMessageType::LevelCmd),
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => check_msg(ButtplugDeviceMessageType::SingleMotorVibrateCmd),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => check_msg(ButtplugDeviceMessageType::StopDeviceCmd),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => check_msg(ButtplugDeviceMessageType::VibrateCmd),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => check_msg(ButtplugDeviceMessageType::VorzeA10CycloneCmd),
    }.map_err(|err| err.into())
  }
  
  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return Box::pin(future::ready(Err(err)));
    }

    Box::pin(future::ready(Ok(messages::Ok::default().into())))
    /*
    let command_array = match command_message {
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => self.handler.handle_fleshlight_launch_fw12_cmd(msg),
      ButtplugDeviceCommandMessageUnion::KiirooCmd(msg) => self.handler.handle_kiiroo_cmd( msg),
      ButtplugDeviceCommandMessageUnion::LevelCmd(msg) => self.handler.handle_level_cmd(msg),
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => self.handler.handle_linear_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawReadCmd(msg) => self.handle_raw_read_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(msg) => self.handle_raw_write_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => self.handler.handle_rotate_cmd(msg),
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(msg) => self.handle_single_motor_vibrate_cmd(msg),
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(msg) => self.stop_commands.clone(),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(msg) => self.handle_raw_subscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(msg) => self.handle_raw_unsubscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => self.handler.handle_vibrate_cmd(msg),
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => self.handler.handle_vorze_a10_cyclone_cmd(msg),
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(msg) => self.handler.handle_battery_level_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(msg) => self.handle_rssi_level_cmd(msg)
    };
    */
  }

  /*
  fn handle_stop_device_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::StopDeviceCmd,
  ) -> ButtplugServerResultFuture {
    let ok_return = messages::Ok::new(message.id());
    let fut_vec: Vec<ButtplugServerResultFuture> = self
      .stop_commands()
      .iter()
      .map(|cmd| self.handle_command(device.clone(), cmd.clone()))
      .collect();
    Box::pin(async move {
      // TODO We should be able to run these concurrently, and should return any error we get.
      for fut in fut_vec {
        if let Err(e) = fut.await {
          error!("{:?}", e);
        }
      }
      Ok(ok_return.into())
    })
  }

  fn handle_single_motor_vibrate_cmd(
    &self,
    message: messages::SingleMotorVibrateCmd,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    // Time for sadness! In order to handle conversion of SingleMotorVibrateCmd, we need to know how
    // many vibrators a device has. We don't actually know that until we get to the protocol level,
    // so we're stuck parsing this here. Since we can assume SingleMotorVibrateCmd will ALWAYS map
    // to vibration, we can convert to VibrateCmd here and save ourselves having to handle it in
    // every protocol, meaning spec v0 and v1 programs will still be forward compatible with
    // vibrators.
    let vibrator_count;
    if let Some(attr) = self
      .attributes
      .message_attributes(&ButtplugDeviceMessageType::VibrateCmd)
    {
      if let Some(count) = attr.feature_count() {
        vibrator_count = *count as usize;
      } else {
        return ButtplugDeviceError::ProtocolRequirementError(format!(
          "{} needs to support VibrateCmd with a feature count to use SingleMotorVibrateCmd.",
          self.name()
        ))
        .into();
      }
    } else {
      return ButtplugDeviceError::ProtocolRequirementError(format!(
        "{} needs to support VibrateCmd to use SingleMotorVibrateCmd.",
        self.name()
      ))
      .into();
    }
    let speed = message.speed();
    let mut cmds = vec![];
    for i in 0..vibrator_count {
      cmds.push(VibrateSubcommand::new(i as u32, speed));
    }
    let mut vibrate_cmd = VibrateCmd::new(message.device_index(), cmds);
    vibrate_cmd.set_id(message.id());
    
  }

  fn handle_raw_write_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawWriteCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.write_value(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_read_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawReadCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.read_value(message.into());
    Box::pin(async move {
      fut.await.map(|mut msg| {
        msg.set_id(id);
        msg.into()
      })
    })
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.unsubscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }

  fn handle_raw_subscribe_cmd(
    &self,
    device: Arc<Hardware>,
    message: messages::RawSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = device.subscribe(message.into());
    Box::pin(async move { fut.await.map(|_| messages::Ok::new(id).into()) })
  }
  */
}

/*
/// Main internal device representation structure
/// 
/// A ButtplugDevice is made up of 2 components:
/// 
/// - A [Device Implementation](crate::device::Hardware), which handles hardware connection and
///   communication.
/// - A [Protocol](crate::device::protocol::ButtplugProtocol), which takes Buttplug Commands and
///   translated them into propreitary commands to send to a device.
/// 
/// When a ButtplugDevice instance is created, it can be assumed that it represents a device that is
/// connected and has been successfully initialized. The instance will manage communication of all
/// commands sent from a [client](crate::client::ButtplugClient) that pertain to this specific
/// hardware.
pub struct ServerDevice {
  /// Protocol instance for the device
  protocol: Box<dyn ButtplugProtocol>,
  /// Hardware implementation for the device
  device: Arc<Hardware>,
  /// Display name for the device
  display_name: OnceCell<String>,
  /// Unique identifier for the device
  device_identifier: ServerDeviceIdentifier
}

impl Debug for ServerDevice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ButtplugDevice")
      .field("name", &self.name())
      .field("identifier", &self.device_identifier())
      .finish()
  }
}

impl Hash for ServerDevice {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.device_identifier().hash(state);
  }
}

impl Eq for ServerDevice {
}

impl PartialEq for ServerDevice {
  fn eq(&self, other: &Self) -> bool {
    self.device_identifier() == other.device_identifier()
  }
}

impl ServerDevice {
  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(protocol: Box<dyn ButtplugProtocol>, device: Arc<Hardware>) -> Self {
    Self {
      device_identifier: ServerDeviceIdentifier::new(device.address(), protocol.protocol_identifier(), protocol.protocol_attributes_identifier()),
      protocol,
      device,
      display_name: OnceCell::new(),
    }
  }

  /// Returns the device identifier
  pub fn device_identifier(&self) -> &ServerDeviceIdentifier {
    &self.device_identifier
  }

  /// Returns the address of the device implementation
  pub fn hardware_address(&self) -> &str {
    self.device.address()
  }

  /// Returns the protocol identifier
  pub fn protocol_identifier(&self) -> &str {
    self.protocol.protocol_identifier()
  }

  /// Returns the protocol attribute identifier
  pub fn protocol_attributes_identifier(&self) -> &ProtocolAttributesIdentifier {
    self.protocol.protocol_attributes_identifier()
  }

  /// Given a possibly usable device, see if any protocol matches. If so, connect and initialize.
  /// 
  /// This is called any time we get a device detection or advertisement from one of our
  /// DeviceCommunicationManager instances. This could be anything from a Bluetooth advertisement,
  /// to detection of a USB device, to one of the various network systems declaring they've found
  /// something. Given the information we've received from that, plus our
  /// [DeviceConfigurationManager](crate::server::device::configuration::DeviceConfigurationManager),
  /// try to find a protocol that has information matching this device. This may include name match,
  /// port matching, etc...
  /// 
  /// If a matching protocol is found, we then call
  /// [ButtplugHardwareCreator::try_create_hardware](crate::device::ButtplugHardwareCreator::try_create_hardware)
  /// with the related protocol information, in order to connect and initialize the device.
  /// 
  /// If all of that is successful, we return a ButtplugDevice that is ready to advertise to the
  /// client and use.
  pub async fn try_create_device(
    protocol_builder: ProtocolInstanceFactory,
    mut device_connector: Box<dyn HardwareConnector>,
  ) -> Result<Option<ServerDevice>, ButtplugError> {
    // TODO This seems needlessly complex, can we clean up how we pass the device builder and protocol factory around?
    
    let mut hardware_specializer = device_connector.connect().await?;

    // Now that we have both a possible device implementation and a configuration for that device,
    // try to initialize the implementation. This usually means trying to connect to whatever the
    // device is, finding endpoints, etc.
    let hardware = hardware_specializer.specialize(&protocol_builder.configuration()).await?;
    info!(
      address = tracing::field::display(hardware.address()),
      "Found Buttplug Device {}",
      hardware.name()
    );

    // If we've made it this far, we now have a connected device implementation with endpoints set
    // up. We now need to run whatever protocol initialization might need to happen. We'll fetch a
    // protocol creator, pass the device implementation to it, then let it do whatever it needs. For
    // most protocols, this is a no-op. However, for devices like Lovense, some Kiiroo, etc, this
    // can get fairly complicated.
    let sharable_hardware = Arc::new(hardware);
    let protocol_impl =
      protocol_builder.create(sharable_hardware.clone()).await?;
    Ok(Some(ServerDevice::new(
      protocol_impl,
      sharable_hardware,
    )))
  }

  /// Get the user created display name for a device, if one exists.
  pub fn display_name(&self) -> Option<String> {
    self.display_name.get().and_then(|name| Some(name.clone()))
  }

  /// Get the name of the device as set in the Device Configuration File.
  /// 
  /// This will also append "(Raw Messaged Allowed)" to the device name if raw mode is on, to warn
  /// users that the device is capable of direct communication.
  pub fn name(&self) -> String {
    // Instead of checking for raw messages at the protocol level, add the raw
    // call here, since this is the only way to access devices in the library
    // anyways.
    //
    // Having raw turned on means it'll work for read/write/sub/unsub on any
    // endpoint so just use an arbitrary message here to check.
    if self
      .protocol
      .supports_message(&ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(
        RawSubscribeCmd::new(1, Endpoint::Tx),
      ))
      .is_ok()
    {
      format!("{} (Raw Messages Allowed)", self.protocol.name())
    } else {
      self.protocol.name().to_owned()
    }
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    self.device.disconnect()
  }

  /// Retreive the message attributes for the device.
  pub fn message_attributes(&self) -> DeviceMessageAttributesMap {
    self.protocol.device_attributes().message_attributes_map()
  }

  /// Parse and route a client command sent for this device.
  pub fn parse_message(
    &self,
    message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    self.protocol.handle_command(self.device.clone(), message)
  }

  /// Retreive the event stream for the device.
  /// 
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
    self.device.event_stream()
  }
}
*/