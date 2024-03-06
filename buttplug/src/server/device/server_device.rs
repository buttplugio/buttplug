// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::{
  fmt::{self, Debug},
  sync::Arc,
  time::Duration,
};

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    message::{
      self,
      ActuatorType,
      BatteryLevelReading,
      ButtplugDeviceCommandMessageUnion,
      ButtplugDeviceMessage,
      ButtplugDeviceMessageType,
      ButtplugMessage,
      ButtplugServerDeviceMessage,
      ButtplugServerMessage,
      Endpoint,
      RSSILevelReading,
      RawReading,
      RawSubscribeCmd,
      ScalarCmd,
      ScalarSubcommand,
      SensorDeviceMessageAttributes,
      SensorReadCmd,
      SensorType,
    },
    ButtplugResultFuture,
  },
  server::{
    device::{
      configuration::{DeviceConfigurationManager, ProtocolAttributesType},
      hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent},
      protocol::ProtocolHandler,
    },
    ButtplugServerResultFuture,
  },
  util::{self, async_manager, stream::convert_broadcast_receiver_to_stream},
};
use core::hash::{Hash, Hasher};
use dashmap::DashSet;
use futures::future::{self, FutureExt};
use getset::{Getters, MutGetters, Setters};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use super::{
  configuration::{ProtocolDeviceAttributes, ServerDeviceMessageAttributes},
  hardware::HardwareWriteCmd,
  protocol::{
    generic_command_manager::GenericCommandManager,
    ProtocolKeepaliveStrategy,
    ProtocolSpecializer,
  },
};

#[derive(Debug)]
pub enum ServerDeviceEvent {
  Connected(Arc<ServerDevice>),
  Notification(ServerDeviceIdentifier, ButtplugServerDeviceMessage),
  Disconnected(ServerDeviceIdentifier),
}

/// Identifying information for a connected devices
///
/// Contains the 3 fields needed to uniquely identify a device in the system.
#[derive(
  Debug, Eq, PartialEq, Hash, Clone, Getters, Setters, MutGetters, Serialize, Deserialize,
)]
#[getset(get = "pub(crate)", get_mut = "pub(crate)")]
pub struct ServerDeviceIdentifier {
  /// Address, as possibly serialized by whatever the managing library for the Device Communication Manager is.
  address: String,
  /// Name of the protocol used
  protocol: String,
  /// Internal identifier for the protocol used
  attributes_identifier: ProtocolAttributesType,
}

impl ServerDeviceIdentifier {
  /// Creates a new instance
  pub fn new(address: &str, protocol: &str, identifier: &ProtocolAttributesType) -> Self {
    Self {
      address: address.to_owned(),
      protocol: protocol.to_owned(),
      attributes_identifier: identifier.clone(),
    }
  }
}

pub(super) async fn build_server_device(
  device_config_manager: Arc<DeviceConfigurationManager>,
  mut hardware_connector: Box<dyn HardwareConnector>,
  protocol_specializers: Vec<ProtocolSpecializer>,
) -> Result<ServerDevice, ButtplugDeviceError> {
  // We've already checked to make sure we have specializers in the server device manager event
  // loop. That check used to be here for sake of continuity in building devices in this method, but
  // having that done before we get here fixes issues with some device advertisement timing (See
  // #462 for more info.)

  // At this point, we know we've got hardware that is waiting to connect, and enough protocol
  // info to actually do something after we connect. So go ahead and connect.
  trace!("Connecting to {:?}", hardware_connector);
  let mut hardware_specializer = hardware_connector.connect().await?;

  // We can't run these in parallel because we need to only accept one specializer.
  let mut protocol_identifier = None;
  let mut hardware_out = None;
  for protocol_specializer in protocol_specializers {
    if let Ok(specialized_hardware) = hardware_specializer
      .specialize(protocol_specializer.specifiers())
      .await
    {
      protocol_identifier = Some(protocol_specializer.identify());
      hardware_out = Some(specialized_hardware);
      break;
    }
  }

  if protocol_identifier.is_none() {
    return Err(ButtplugDeviceError::DeviceConfigurationError(
      "No protocols with viable communication matches for hardware.".to_owned(),
    ));
  }

  let mut protocol_identifier_stage = protocol_identifier.unwrap();
  let hardware = Arc::new(hardware_out.unwrap());

  let (identifier, mut protocol_initializer) =
    protocol_identifier_stage.identify(hardware.clone()).await?;

  // Now we have an identifier. After this point, if anything fails, consider it a complete
  // connection failure, as identify may have already run commands on the device, and therefore
  // put it in an unknown state if anything fails.

  // Check in the DeviceConfigurationManager to make sure we have attributes
  // for this device.
  let attrs = if let Some(attrs) =
    device_config_manager.protocol_device_attributes(&identifier, &hardware.endpoints())
  {
    attrs
  } else {
    return Err(ButtplugDeviceError::DeviceConfigurationError(format!(
      "No protocols with viable protocol attributes for hardware {:?}.",
      identifier
    )));
  };

  // If we have attributes, go ahead and initialize, handing us back our hardware instance that
  // is now ready to use with the protocol handler.

  // Build the server device and return.

  let handler = protocol_initializer
    .initialize(hardware.clone(), &attrs)
    .await?;

  let requires_keepalive = hardware.requires_keepalive();
  let strategy = handler.keepalive_strategy();

  // We now have fully initialized hardware, return a server device.
  let device = ServerDevice::new(identifier, handler, hardware, &attrs);

  // If we need a keepalive with a packet replay, set this up via stopping the device on connect.
  if requires_keepalive
    && matches!(
      strategy,
      ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
    )
  {
    if let Err(e) = device.handle_stop_device_cmd().await {
      return Err(ButtplugDeviceError::DeviceConnectionError(format!(
        "Error setting up keepalive: {}",
        e
      )));
    }
  }

  Ok(device)
}

pub struct ServerDevice {
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  attributes: ProtocolDeviceAttributes,
  generic_command_manager: GenericCommandManager,
  /// Unique identifier for the device
  identifier: ServerDeviceIdentifier,
  raw_subscribed_endpoints: Arc<DashSet<Endpoint>>,
  keepalive_packet: Arc<RwLock<Option<HardwareWriteCmd>>>,
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
  fn new(
    identifier: ServerDeviceIdentifier,
    handler: Arc<dyn ProtocolHandler>,
    hardware: Arc<Hardware>,
    attributes: &ProtocolDeviceAttributes,
  ) -> Self {
    let keepalive_packet = Arc::new(RwLock::new(None));
    let gcm = GenericCommandManager::new(attributes);
    // If we've gotten here, we know our hardware is connected. This means we can start the keepalive if it's required.
    if hardware.requires_keepalive()
      && !matches!(
        handler.keepalive_strategy(),
        ProtocolKeepaliveStrategy::NoStrategy
      )
    {
      let hardware = hardware.clone();
      let strategy = handler.keepalive_strategy();
      let keepalive_packet = keepalive_packet.clone();
      async_manager::spawn(async move {
        // Arbitrary wait time for now.
        let wait_duration = Duration::from_secs(5);
        loop {
          if hardware.time_since_last_write().await > wait_duration {
            match &strategy {
              ProtocolKeepaliveStrategy::RepeatPacketStrategy(packet) => {
                if let Err(e) = hardware.write_value(&packet).await {
                  warn!("Error writing keepalive packet: {:?}", e);
                  break;
                }
              }
              ProtocolKeepaliveStrategy::RepeatLastPacketStrategy => {
                if let Some(packet) = &*keepalive_packet.read().await {
                  if let Err(e) = hardware.write_value(&packet).await {
                    warn!("Error writing keepalive packet: {:?}", e);
                    break;
                  }
                }
              }
              _ => {
                info!(
                  "Protocol keepalive strategy {:?} not implemented, replacing with NoStrategy",
                  strategy
                );
              }
            }
          }
          // Arbitrary wait time for now.
          util::sleep(wait_duration).await;
        }
        info!("Leaving keepalive task for {}", hardware.name());
      });
    }

    Self {
      identifier,
      generic_command_manager: gcm,
      handler,
      hardware,
      keepalive_packet,
      attributes: attributes.clone(),
      raw_subscribed_endpoints: Arc::new(DashSet::new()),
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
    let fut = self.hardware.disconnect();
    async move { fut.await.map_err(|err| err.into()) }.boxed()
  }

  /// Retreive the message attributes for the device.
  pub fn message_attributes(&self) -> ServerDeviceMessageAttributes {
    self.attributes.message_attributes()
  }

  /// Retreive the event stream for the device.
  ///
  /// This will include connections, disconnections, and notification events from subscribed
  /// endpoints.
  pub fn event_stream(&self) -> impl futures::Stream<Item = ServerDeviceEvent> + Send {
    let identifier = self.identifier.clone();
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    let hardware_stream = convert_broadcast_receiver_to_stream(self.hardware.event_stream())
      .filter_map(move |hardware_event| {
        let id = identifier.clone();
        match hardware_event {
          HardwareEvent::Disconnected(_) => Some(ServerDeviceEvent::Disconnected(id)),
          HardwareEvent::Notification(_address, endpoint, data) => {
            // TODO Figure out how we're going to parse raw data into something sendable to the client.
            if raw_endpoints.contains(&endpoint) {
              Some(ServerDeviceEvent::Notification(
                id,
                ButtplugServerDeviceMessage::RawReading(RawReading::new(0, endpoint, data)),
              ))
            } else {
              None
            }
          }
        }
      });

    let identifier = self.identifier.clone();
    let handler_mapped_stream = self.handler.event_stream().map(move |incoming_message| {
      let id = identifier.clone();
      ServerDeviceEvent::Notification(id, incoming_message)
    });
    hardware_stream.merge(handler_mapped_stream)
  }

  pub fn supports_message(
    &self,
    message: &ButtplugDeviceCommandMessageUnion,
  ) -> Result<(), ButtplugError> {
    // TODO This should be generated by a macro, as should the types enum.
    let check_msg = |msg_type| {
      self
        .attributes
        .allows_message(&msg_type)
        .then_some(())
        .ok_or(ButtplugDeviceError::MessageNotSupported(msg_type))
    };

    match message {
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => {
        check_msg(ButtplugDeviceMessageType::BatteryLevelCmd)
      }
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(_) => {
        check_msg(ButtplugDeviceMessageType::FleshlightLaunchFW12Cmd)
      }
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => {
        check_msg(ButtplugDeviceMessageType::KiirooCmd)
      }
      ButtplugDeviceCommandMessageUnion::LinearCmd(_) => {
        check_msg(ButtplugDeviceMessageType::LinearCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawReadCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawReadCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawSubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawUnsubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RawWriteCmd)
      }
      ButtplugDeviceCommandMessageUnion::RotateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RotateCmd)
      }
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => {
        check_msg(ButtplugDeviceMessageType::RSSILevelCmd)
      }
      ButtplugDeviceCommandMessageUnion::ScalarCmd(_) => {
        check_msg(ButtplugDeviceMessageType::ScalarCmd)
      }
      // We translate SingleMotorVibrateCmd into Vibrate, so this one is special.
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::VibrateCmd)
      }
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => {
        check_msg(ButtplugDeviceMessageType::StopDeviceCmd)
      }
      ButtplugDeviceCommandMessageUnion::VibrateCmd(_) => {
        check_msg(ButtplugDeviceMessageType::VibrateCmd)
      }
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(_) => {
        check_msg(ButtplugDeviceMessageType::VorzeA10CycloneCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorReadCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorReadCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorSubscribeCmd)
      }
      ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(_) => {
        check_msg(ButtplugDeviceMessageType::SensorUnsubscribeCmd)
      }
    }
    .map_err(|err| err.into())
  }

  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnion,
  ) -> ButtplugServerResultFuture {
    if let Err(err) = self.supports_message(&command_message) {
      return future::ready(Err(err)).boxed();
    }

    // If a handler implements handle message, bypass all of our parsing and let it do its own
    // thing. This should be a very rare thing.
    if self.handler.has_handle_message() {
      let fut = self.handle_generic_command_result(self.handler.handle_message(&command_message));
      return async move { fut.await }.boxed();
    }

    match command_message {
      // messages we can handle in this struct
      ButtplugDeviceCommandMessageUnion::RawReadCmd(msg) => self.handle_raw_read_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawWriteCmd(msg) => self.handle_raw_write_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawSubscribeCmd(msg) => self.handle_raw_subscribe_cmd(msg),
      ButtplugDeviceCommandMessageUnion::RawUnsubscribeCmd(msg) => {
        self.handle_raw_unsubscribe_cmd(msg)
      }
      ButtplugDeviceCommandMessageUnion::StopDeviceCmd(_) => self.handle_stop_device_cmd(),
      ButtplugDeviceCommandMessageUnion::SingleMotorVibrateCmd(msg) => {
        self.handle_single_motor_vibrate_cmd(msg)
      }
      ButtplugDeviceCommandMessageUnion::BatteryLevelCmd(_) => self.handle_battery_level_cmd(),
      ButtplugDeviceCommandMessageUnion::RSSILevelCmd(_) => self.handle_rssi_level_cmd(),
      // Message that return lists of hardware commands which we'll handle sending to the devices
      // here, in order to reduce boilerplate in the implementations. Generic messages that we can
      // use the generic command manager for, but still need protocol level translation.
      ButtplugDeviceCommandMessageUnion::ScalarCmd(msg) => {
        // TODO Add ability to turn off actuator matching
        let attributes = self.attributes.message_attributes();
        let attrs = attributes
          .scalar_cmd()
          .as_ref()
          .expect("Already checked existence");
        for command in msg.scalars() {
          if command.index() > attrs.len() as u32 {
            return future::ready(Err(
              ButtplugDeviceError::DeviceFeatureIndexError(attrs.len() as u32, command.index())
                .into(),
            ))
            .boxed();
          }
          if *attrs[command.index() as usize].actuator_type() != command.actuator_type() {
            return future::ready(Err(
              ButtplugDeviceError::DeviceActuatorTypeMismatch(
                self.name(),
                command.actuator_type(),
                *attrs[command.index() as usize].actuator_type(),
              )
              .into(),
            ))
            .boxed();
          }
        }

        let commands = match self
          .generic_command_manager
          .update_scalar(&msg, self.handler.needs_full_command_set())
        {
          Ok(values) => values,
          Err(err) => return future::ready(Err(err)).boxed(),
        };

        if commands.is_empty() {
          trace!(
            "No commands generated for incoming device packet, skipping and returning success."
          );
          return future::ready(Ok(message::Ok::default().into())).boxed();
        }

        self.handle_generic_command_result(self.handler.handle_scalar_cmd(&commands))
      }
      ButtplugDeviceCommandMessageUnion::RotateCmd(msg) => {
        let commands = match self
          .generic_command_manager
          .update_rotation(&msg, self.handler.needs_full_command_set())
        {
          Ok(values) => values,
          Err(err) => return future::ready(Err(err)).boxed(),
        };
        self.handle_generic_command_result(self.handler.handle_rotate_cmd(&commands))
      }
      ButtplugDeviceCommandMessageUnion::VibrateCmd(msg) => {
        self.parse_message(ScalarCmd::from(msg).into())
      }
      ButtplugDeviceCommandMessageUnion::LinearCmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_linear_cmd(msg))
      }
      ButtplugDeviceCommandMessageUnion::FleshlightLaunchFW12Cmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_fleshlight_launch_fw12_cmd(msg))
      }
      ButtplugDeviceCommandMessageUnion::VorzeA10CycloneCmd(msg) => {
        self.handle_generic_command_result(self.handler.handle_vorze_a10_cyclone_cmd(msg))
      }
      ButtplugDeviceCommandMessageUnion::SensorReadCmd(msg) => self.handle_sensor_read_cmd(msg),
      ButtplugDeviceCommandMessageUnion::SensorSubscribeCmd(msg) => {
        self.handle_sensor_subscribe_cmd(msg)
      }
      ButtplugDeviceCommandMessageUnion::SensorUnsubscribeCmd(msg) => {
        self.handle_sensor_unsubscribe_cmd(msg)
      }
      // Everything else, which is mostly older messages, or special things that require reads.
      ButtplugDeviceCommandMessageUnion::KiirooCmd(_) => future::ready(Err(
        ButtplugDeviceError::ProtocolNotImplemented("Being Lazy".to_owned()).into(),
      ))
      .boxed(), //self.handler.handle_kiiroo_cmd( msg),
    }
  }

  fn handle_hardware_commands(&self, commands: Vec<HardwareCommand>) -> ButtplugServerResultFuture {
    let hardware = self.hardware.clone();
    let keepalive_type = self.handler.keepalive_strategy();
    let keepalive_packet = self.keepalive_packet.clone();
    async move {
      // Run commands in order, otherwise we may end up sending out of order. This may take a while,
      // but it's what 99% of protocols expect. If they want something else, they can implement it
      // themselves.
      //
      // If anything errors out, just bail on the command series. This most likely means the device
      // disconnected.
      for command in commands {
        hardware.parse_message(&command).await?;
        if hardware.requires_keepalive()
          && matches!(
            keepalive_type,
            ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
          )
        {
          if let HardwareCommand::Write(command) = command {
            *keepalive_packet.write().await = Some(command);
          }
        }
      }
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  fn handle_generic_command_result(
    &self,
    command_result: Result<Vec<HardwareCommand>, ButtplugDeviceError>,
  ) -> ButtplugServerResultFuture {
    let hardware_commands = match command_result {
      Ok(commands) => commands,
      Err(err) => return future::ready(Err(err.into())).boxed(),
    };

    self.handle_hardware_commands(hardware_commands)
  }

  fn handle_stop_device_cmd(&self) -> ButtplugServerResultFuture {
    let commands = self.generic_command_manager.stop_commands();
    let mut fut_vec = vec![];
    commands
      .iter()
      .for_each(|msg| fut_vec.push(self.parse_message(msg.clone())));
    async move {
      for fut in fut_vec {
        fut.await?;
      }
      Ok(message::Ok::default().into())
    }
    .boxed()
  }

  fn check_sensor_command(
    &self,
    attributes: &Vec<SensorDeviceMessageAttributes>,
    sensor_index: &u32,
    sensor_type: &SensorType,
  ) -> Result<(), ButtplugDeviceError> {
    if let Some(sensor_info) = attributes.get(*sensor_index as usize) {
      if *sensor_info.sensor_type() == *sensor_type {
        Ok(())
      } else {
        Err(ButtplugDeviceError::DeviceSensorTypeMismatch(
          *sensor_index,
          *sensor_type,
          *sensor_info.sensor_type(),
        ))
      }
    } else {
      Err(ButtplugDeviceError::DeviceSensorIndexError(
        attributes.len() as u32,
        *sensor_index,
      ))
    }
  }

  fn handle_sensor_read_cmd(&self, message: message::SensorReadCmd) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(
      self
        .message_attributes()
        .sensor_read_cmd()
        .as_ref()
        .expect("Already checked validity"),
      message.sensor_index(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_read_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_subscribe_cmd(
    &self,
    message: message::SensorSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(
      self
        .message_attributes()
        .sensor_subscribe_cmd()
        .as_ref()
        .expect("Already checked validity"),
      message.sensor_index(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_subscribe_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd(
    &self,
    message: message::SensorUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let result = self.check_sensor_command(
      self
        .message_attributes()
        .sensor_subscribe_cmd()
        .as_ref()
        .expect("Already checked validity"),
      message.sensor_index(),
      message.sensor_type(),
    );
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      result?;
      handler
        .handle_sensor_unsubscribe_cmd(device, message)
        .await
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_single_motor_vibrate_cmd(
    &self,
    message: message::SingleMotorVibrateCmd,
  ) -> ButtplugServerResultFuture {
    if let Some(attr) = self.attributes.message_attributes().scalar_cmd() {
      let speed = message.speed();
      let cmds: Vec<ScalarSubcommand> = attr
        .iter()
        .enumerate()
        .filter(|(_, x)| *x.actuator_type() == ActuatorType::Vibrate)
        .map(|(index, _)| ScalarSubcommand::new(index as u32, speed, ActuatorType::Vibrate))
        .collect();
      if cmds.is_empty() {
        ButtplugDeviceError::ProtocolRequirementError(format!(
          "{} has no vibrating features.",
          self.name()
        ))
        .into()
      } else {
        let mut vibrate_cmd = ScalarCmd::new(message.device_index(), cmds);
        vibrate_cmd.set_id(message.id());
        self.parse_message(vibrate_cmd.into())
      }
    } else {
      ButtplugDeviceError::ProtocolRequirementError(format!(
        "{} needs to support ScalarCmd to use SingleMotorVibrateCmd.",
        self.name()
      ))
      .into()
    }
  }

  fn handle_raw_write_cmd(&self, message: message::RawWriteCmd) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.write_value(&message.into());
    async move {
      fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_read_cmd(&self, message: message::RawReadCmd) -> ButtplugServerResultFuture {
    let id = message.id();
    let fut = self.hardware.read_value(&message.into());
    async move {
      fut
        .await
        .map(|msg| {
          let mut raw_msg: RawReading = msg.into();
          raw_msg.set_id(id);
          raw_msg.into()
        })
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_unsubscribe_cmd(
    &self,
    message: message::RawUnsubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.unsubscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if !raw_endpoints.contains(&endpoint) {
        return Ok(message::Ok::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.remove(&endpoint);
      result
    }
    .boxed()
  }

  fn handle_raw_subscribe_cmd(
    &self,
    message: message::RawSubscribeCmd,
  ) -> ButtplugServerResultFuture {
    let id = message.id();
    let endpoint = message.endpoint();
    let fut = self.hardware.subscribe(&message.into());
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if raw_endpoints.contains(&endpoint) {
        return Ok(message::Ok::new(id).into());
      }
      let result = fut
        .await
        .map(|_| message::Ok::new(id).into())
        .map_err(|err| err.into());
      raw_endpoints.insert(endpoint);
      result
    }
    .boxed()
  }

  fn handle_battery_level_cmd(&self) -> ButtplugServerResultFuture {
    // See if we have a battery sensor.
    if let Some(sensor_attributes) = self.message_attributes().sensor_read_cmd() {
      for (index, sensor) in sensor_attributes.iter().enumerate() {
        if *sensor.sensor_type() == SensorType::Battery {
          let sensor_read_msg = SensorReadCmd::new(0, index as u32, SensorType::Battery);
          let sensor_read = self.handle_sensor_read_cmd(sensor_read_msg);
          let sensor_range_end = *sensor.sensor_range()[0].end();
          return async move {
            let return_msg = sensor_read.await?;
            if let ButtplugServerMessage::SensorReading(reading) = return_msg {
              if reading.sensor_type() == SensorType::Battery {
                Ok(
                  BatteryLevelReading::new(0, reading.data()[0] as f64 / sensor_range_end as f64)
                    .into(),
                )
              } else {
                Err(ButtplugError::ButtplugDeviceError(
                  ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::Battery),
                ))
              }
            } else {
              Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::Battery),
              ))
            }
          }
          .boxed();
        }
      }
    }
    future::ready(Err(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::Battery),
    )))
    .boxed()
  }

  fn handle_rssi_level_cmd(&self) -> ButtplugServerResultFuture {
    // See if we have a battery sensor.
    if let Some(sensor_attributes) = self.message_attributes().sensor_read_cmd() {
      for (index, sensor) in sensor_attributes.iter().enumerate() {
        if *sensor.sensor_type() == SensorType::RSSI {
          let sensor_read_msg = SensorReadCmd::new(0, index as u32, SensorType::RSSI);
          let sensor_read = self.handle_sensor_read_cmd(sensor_read_msg);
          return async move {
            let return_msg = sensor_read.await?;
            if let ButtplugServerMessage::SensorReading(reading) = return_msg {
              if reading.sensor_type() == SensorType::RSSI {
                Ok(RSSILevelReading::new(0, reading.data()[0]).into())
              } else {
                Err(ButtplugError::ButtplugDeviceError(
                  ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::RSSI),
                ))
              }
            } else {
              Err(ButtplugError::ButtplugDeviceError(
                ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::RSSI),
              ))
            }
          }
          .boxed();
        }
      }
    }
    future::ready(Err(ButtplugError::ButtplugDeviceError(
      ButtplugDeviceError::ProtocolSensorNotSupported(SensorType::RSSI),
    )))
    .boxed()
  }
}
