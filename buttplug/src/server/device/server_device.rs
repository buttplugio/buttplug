// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Server Device Implementation
//!
//! This struct manages the trip from buttplug protocol actuator/sensor/raw message to hardware
//! communication. This involves:
//!
//! - Taking buttplug device command messages from the exposed server
//! - Converting older spec version messages to the newest spec version, which usually requires
//!   device information for actuation/sensor messages.
//! - Validity checking the messages to make sure they match the capabilities of the hardware
//! - Turning the buttplug messages into hardware commands via the associated protocol
//! - Sending them to the hardware
//! - Possibly receiving back information (in the case of sensors), possibly firing and forgetting
//!   (in terms of almost everything else)
//!
//! We make a lot of assumptions in here based on the devices we support right now, including:
//!
//! - Devices will only ever have one directional rotation actuator (we have no device that supports
//!   two rotational components currently)
//! - Devices will only ever have one linear actuator (we have no device that supports multiple
//!   linear actuators currently)
//! - Devices scalar command ordering is explicitly set by the device config file
//!   - This means that we rely on the config file to know which vibrator is which on a device with
//!     multiple vibrators. In protocols, especially for toy brands that release a large line of
//!     different toys all using the same protocols (lovense, wevibe, etc), the order of features in
//!     the config file MATTERS and needs to be tested against an actual device to make sure we're
//!     controlling the actuator we think we are.
//!   - This situation sucks and we should have better definitions, a problem outlined at
//!     https://github.com/buttplugio/buttplug/issues/646
//!
//! In order to handle multiple message spec versions

use std::{
  collections::VecDeque,
  fmt::{self, Debug},
  sync::Arc,
  time::Duration,
};

use crate::{
  core::{
    errors::{ButtplugDeviceError, ButtplugError},
    message::{
      self,
      OutputRotateWithDirection,
      OutputType,
      OutputValue,
      ButtplugMessage,
      ButtplugServerMessageV4,
      DeviceFeature,
      DeviceMessageInfoV4,
      Endpoint,
      RawCommand,
      RawCommandRead,
      RawCommandWrite,
      RawReadingV2,
      InputCommandType,
      InputType,
    },
    ButtplugResultFuture,
  },
  server::{
    device::{
      configuration::DeviceConfigurationManager,
      hardware::{
        Hardware,
        HardwareCommand,
        HardwareConnector,
        HardwareEvent,
        HardwareReadCmd,
        HardwareSubscribeCmd,
        HardwareUnsubscribeCmd,
        HardwareWriteCmd,
        GENERIC_RAW_COMMAND_UUID,
      },
      protocol::ProtocolHandler,
    },
    message::{
      checked_output_cmd::CheckedOutputCmdV4,
      checked_raw_cmd::CheckedRawCmdV4,
      checked_input_cmd::CheckedInputCmdV4,
      server_device_attributes::ServerDeviceAttributes,
      spec_enums::ButtplugDeviceCommandMessageUnionV4,
      ButtplugServerDeviceMessage,
    },
    ButtplugServerResultFuture,
  },
  util::{self, async_manager, stream::convert_broadcast_receiver_to_stream},
};
use core::hash::{Hash, Hasher};
use dashmap::{DashMap, DashSet};
use futures::future::{self, BoxFuture, FutureExt};
use getset::Getters;
use tokio::sync::{Mutex, RwLock};
use tokio_stream::StreamExt;
use uuid::Uuid;

use super::{
  configuration::{UserDeviceDefinition, UserDeviceIdentifier},
  protocol::{
    //output_command_manager::ActuatorCommandManager,
    ProtocolKeepaliveStrategy,
    ProtocolSpecializer,
  },
};

#[derive(Debug)]
pub enum ServerDeviceEvent {
  Connected(Arc<ServerDevice>),
  Notification(UserDeviceIdentifier, ButtplugServerDeviceMessage),
  Disconnected(UserDeviceIdentifier),
}

#[derive(Getters)]
pub struct ServerDevice {
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  #[getset(get = "pub")]
  definition: UserDeviceDefinition,
  //output_command_manager: ActuatorCommandManager,
  /// Unique identifier for the device
  #[getset(get = "pub")]
  identifier: UserDeviceIdentifier,
  raw_subscribed_endpoints: Arc<DashSet<Endpoint>>,
  #[getset(get = "pub")]
  legacy_attributes: ServerDeviceAttributes,
  last_output_command: DashMap<Uuid, CheckedOutputCmdV4>,
  current_hardware_commands: Arc<Mutex<Option<VecDeque<HardwareCommand>>>>,
  stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4>,
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
  pub(super) async fn build(
    device_config_manager: Arc<DeviceConfigurationManager>,
    mut hardware_connector: Box<dyn HardwareConnector>,
    protocol_specializers: Vec<ProtocolSpecializer>,
  ) -> Result<Self, ButtplugDeviceError> {
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

    let (identifier, mut protocol_initializer) = protocol_identifier_stage
      .identify(hardware.clone(), hardware_connector.specifier())
      .await?;

    // Now we have an identifier. After this point, if anything fails, consider it a complete
    // connection failure, as identify may have already run commands on the device, and therefore
    // put it in an unknown state if anything fails.

    // Check in the DeviceConfigurationManager to make sure we have attributes for this device.
    let attrs = if let Some(attrs) =
      device_config_manager.device_definition(&identifier, &hardware.endpoints())
    {
      attrs
    } else {
      return Err(ButtplugDeviceError::DeviceConfigurationError(format!(
        "No protocols with viable protocol attributes for hardware {identifier:?}."
      )));
    };

    // If we have attributes, go ahead and initialize, handing us back our hardware instance that
    // is now ready to use with the protocol handler.

    // Build the server device and return.
    let handler = protocol_initializer
      .initialize(hardware.clone(), &attrs.clone())
      .await?;

    let requires_keepalive = hardware.requires_keepalive();
    let strategy = handler.keepalive_strategy();

    // We now have fully initialized hardware, return a server device.
    let device = Self::new(identifier, handler, hardware, &attrs);

    // If we need a keepalive with a packet replay, set this up via stopping the device on connect.
    if requires_keepalive
      && matches!(
        strategy,
        ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
      )
    {
      if let Err(e) = device.handle_stop_device_cmd().await {
        return Err(ButtplugDeviceError::DeviceConnectionError(format!(
          "Error setting up keepalive: {e}"
        )));
      }
    }

    Ok(device)
  }

  /// Given a protocol and a device impl, create a new ButtplugDevice instance
  fn new(
    identifier: UserDeviceIdentifier,
    handler: Arc<dyn ProtocolHandler>,
    hardware: Arc<Hardware>,
    definition: &UserDeviceDefinition,
  ) -> Self {
    let keepalive_packet = Arc::new(RwLock::new(None));
    let current_hardware_commands = Arc::new(Mutex::new(None));

    // Set up and start the packet send task
    {
      let current_hardware_commands = current_hardware_commands.clone();
      let hardware = hardware.clone();
      let strategy = handler.keepalive_strategy();
      let keepalive_packet = keepalive_packet.clone();
      async_manager::spawn(async move {
        // Arbitrary wait time for now.
        let wait_duration = Duration::from_secs(5);
        let bt_duration = Duration::from_millis(75);
        loop {
          // Loop based on our 10hz estimate for most BLE toys.
          util::sleep(bt_duration).await;
          // Run commands in order, otherwise we may end up sending out of order. This may take a while,
          // but it's what 99% of protocols expect. If they want something else, they can implement it
          // themselves.
          //
          // If anything errors out, just bail on the command series. This most likely means the device
          // disconnected.
          let mut local_commands: VecDeque<HardwareCommand> = {
            let mut c = current_hardware_commands.lock().await;
            if let Some(command_vec) = c.take() {
              command_vec
            } else {
              continue;
            }
          };
          while let Some(command) = local_commands.pop_front() {
            debug!("Sending hardware command {:?}", command);
            // TODO This needs to throw system error messages
            let _ = hardware.parse_message(&command).await;
            if hardware.requires_keepalive()
              && matches!(
                strategy,
                ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
              )
            {
              if let HardwareCommand::Write(command) = command {
                *keepalive_packet.write().await = Some(command);
              }
            }
          }
          if hardware.requires_keepalive()
            && !matches!(strategy, ProtocolKeepaliveStrategy::NoStrategy)
            && hardware.time_since_last_write().await > wait_duration
          {
            match &strategy {
              ProtocolKeepaliveStrategy::RepeatPacketStrategy(packet) => {
                if let Err(e) = hardware.write_value(packet).await {
                  warn!("Error writing keepalive packet: {:?}", e);
                  break;
                }
              }
              ProtocolKeepaliveStrategy::RepeatLastPacketStrategy => {
                if let Some(packet) = &*keepalive_packet.read().await {
                  if let Err(e) = hardware.write_value(packet).await {
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
        }
        info!("Leaving keepalive task for {}", hardware.name());
      });
    }

    let mut stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4> = vec![];
    // We consider the feature's FeatureType to be the "main" capability of a feature. Use that to
    // calculate stop commands.
    for (index, feature) in definition.features().iter().enumerate() {
      if let Some(output_map) = feature.output() {
        for actuator_type in output_map.keys() {
          let mut stop_cmd = |actuator_cmd| {
            stop_commands.push(
              CheckedOutputCmdV4::new(1, 0, index as u32, feature.id(), actuator_cmd).into(),
            );
          };

          // Break out of these if one is found, we only need 1 stop message per output.
          match actuator_type {
            OutputType::Constrict => {
              stop_cmd(message::OutputCommand::Constrict(OutputValue::new(0)));
              break;
            }
            OutputType::Heater => {
              stop_cmd(message::OutputCommand::Heater(OutputValue::new(0)));
              break;
            }
            OutputType::Inflate => {
              stop_cmd(message::OutputCommand::Inflate(OutputValue::new(0)));
              break;
            }
            OutputType::Led => {
              stop_cmd(message::OutputCommand::Led(OutputValue::new(0)));
              break;
            }
            OutputType::Oscillate => {
              stop_cmd(message::OutputCommand::Oscillate(OutputValue::new(0)));
              break;
            }
            OutputType::Rotate => {
              stop_cmd(message::OutputCommand::Rotate(OutputValue::new(0)));
              break;
            }
            OutputType::RotateWithDirection => {
              stop_cmd(message::OutputCommand::RotateWithDirection(
                OutputRotateWithDirection::new(0, true),
              ));
              break;
            }
            OutputType::Vibrate => {
              stop_cmd(message::OutputCommand::Vibrate(OutputValue::new(0)));
              break;
            }
            _ => {
              // There's not much we can do about position or position w/ duration, so just continue on
              continue;
            }
          }
        }
      }
    }
    Self {
      identifier,
      //output_command_manager: acm,
      handler,
      hardware,
      definition: definition.clone(),
      raw_subscribed_endpoints: Arc::new(DashSet::new()),
      // Generating legacy attributes is cheap, just do it right when we create the device.
      legacy_attributes: ServerDeviceAttributes::new(definition.features()),
      last_output_command: DashMap::new(),
      current_hardware_commands,
      stop_commands,
    }
  }

  /// Get the name of the device as set in the Device Configuration File.
  pub fn name(&self) -> String {
    self.definition.name().to_owned()
  }

  /// Disconnect from the device, if it's connected.
  pub fn disconnect(&self) -> ButtplugResultFuture {
    let fut = self.hardware.disconnect();
    async move { fut.await.map_err(|err| err.into()) }.boxed()
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
                ButtplugServerDeviceMessage::RawReading(RawReadingV2::new(0, endpoint, data)),
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

  pub fn needs_update(&self, _command_message: &ButtplugDeviceCommandMessageUnionV4) -> bool {
    true
  }

  pub fn as_device_message_info(&self, index: u32) -> DeviceMessageInfoV4 {
    DeviceMessageInfoV4::new(
      index,
      &self.name(),
      self.definition().user_config().display_name(),
      100,
      &self
        .definition
        .features()
        .iter()
        .enumerate()
        .map(|(i, x)| x.as_device_feature(i as u32))
        .collect::<Vec<DeviceFeature>>(),
    )
  }

  // In order to not have to worry about id setting at the protocol level (this
  // should be taken care of in the server's device manager), we return server
  // messages but Buttplug errors.
  pub fn parse_message(
    &self,
    command_message: ButtplugDeviceCommandMessageUnionV4,
  ) -> ButtplugServerResultFuture {
    match command_message {
      // Raw messages
      ButtplugDeviceCommandMessageUnionV4::RawCmd(msg) => self.handle_raw_cmd(msg),
      // Sensor messages
      ButtplugDeviceCommandMessageUnionV4::SensorCmd(msg) => self.handle_sensor_cmd(msg),
      // Actuator messages
      ButtplugDeviceCommandMessageUnionV4::ActuatorCmd(msg) => self.handle_actuatorcmd_v4(&msg),
      ButtplugDeviceCommandMessageUnionV4::ActuatorVecCmd(msg) => {
        let mut futs = vec![];
        let msg_id = msg.id();
        for m in msg.value_vec() {
          futs.push(self.handle_actuatorcmd_v4(m))
        }
        async move {
          for f in futs {
            f.await?;
          }
          Ok(message::OkV0::new(msg_id).into())
        }
        .boxed()
      }
      // Other generic messages
      ButtplugDeviceCommandMessageUnionV4::StopDeviceCmd(_) => self.handle_stop_device_cmd(),
    }
  }

  fn handle_actuatorcmd_v4(&self, msg: &CheckedOutputCmdV4) -> ButtplugServerResultFuture {
    if let Some(last_msg) = self.last_output_command.get(&msg.feature_id()) {
      if *last_msg == *msg {
        trace!("No commands generated for incoming device packet, skipping and returning success.");
        return future::ready(Ok(message::OkV0::default().into())).boxed();
      }
    }
    self
      .last_output_command
      .insert(msg.feature_id(), msg.clone());
    self.handle_generic_command_result(self.handler.handle_output_cmd(msg))
  }

  fn handle_hardware_commands(&self, commands: Vec<HardwareCommand>) -> ButtplugServerResultFuture {
    let current_hardware_commands = self.current_hardware_commands.clone();
    async move {
      // Run commands in order, otherwise we may end up sending out of order. This may take a while,
      // but it's what 99% of protocols expect. If they want something else, they can implement it
      // themselves.
      //
      // If anything errors out, just bail on the command series. This most likely means the device
      // disconnected.
      let mut c = current_hardware_commands.lock().await;
      if let Some(g) = c.as_mut() {
        for command in commands {
          g.retain(|v| v.command_id() != command.command_id());
          g.push_back(command);
        }
      } else {
        let mut n = VecDeque::new();
        for command in commands {
          n.push_back(command);
        }
        *c = Some(n);
      }
      Ok(message::OkV0::default().into())
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
    let mut fut_vec = vec![];
    self
      .stop_commands
      .iter()
      .for_each(|msg| fut_vec.push(self.parse_message(msg.clone())));
    async move {
      for fut in fut_vec {
        fut.await?;
      }
      Ok(message::OkV0::default().into())
    }
    .boxed()
  }

  fn handle_sensor_cmd(
    &self,
    message: CheckedInputCmdV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    match message.input_command() {
      InputCommandType::Read => self.handle_sensor_read_cmd(
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Subscribe => self.handle_sensor_subscribe_cmd(
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Unsubscribe => self.handle_sensor_unsubscribe_cmd(
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
    }
  }

  fn handle_sensor_read_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_read_cmd(device, feature_index, feature_id, sensor_type)
        .await
        .map_err(|e| e.into())
        .map(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_subscribe_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> ButtplugServerResultFuture {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_subscribe_cmd(device, feature_index, feature_id, sensor_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_sensor_unsubscribe_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    sensor_type: InputType,
  ) -> ButtplugServerResultFuture {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_unsubscribe_cmd(device, feature_index, feature_id, sensor_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_raw_cmd(&self, message: CheckedRawCmdV4) -> ButtplugServerResultFuture {
    match message.raw_command() {
      RawCommand::Read(read_data) => self.handle_raw_read_cmd(*message.endpoint(), read_data),
      RawCommand::Subscribe => self.handle_raw_subscribe_cmd(*message.endpoint()),
      RawCommand::Unsubscribe => self.handle_raw_unsubscribe_cmd(*message.endpoint()),
      RawCommand::Write(write_data) => self.handle_raw_write_cmd(*message.endpoint(), write_data),
    }
  }

  fn handle_raw_write_cmd(
    &self,
    endpoint: Endpoint,
    write_data: &RawCommandWrite,
  ) -> ButtplugServerResultFuture {
    let fut = self.hardware.write_value(&HardwareWriteCmd::new(
      GENERIC_RAW_COMMAND_UUID,
      endpoint,
      write_data.data().clone(),
      write_data.write_with_response(),
    ));
    async move {
      fut
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_read_cmd(
    &self,
    endpoint: Endpoint,
    read_data: &RawCommandRead,
  ) -> ButtplugServerResultFuture {
    let fut = self.hardware.read_value(&HardwareReadCmd::new(
      GENERIC_RAW_COMMAND_UUID,
      endpoint,
      read_data.expected_length(),
      read_data.timeout(),
    ));
    async move {
      fut
        .await
        .map(|msg| {
          let mut raw_msg: RawReadingV2 = msg.into();
          raw_msg.set_id(1);
          raw_msg.into()
        })
        .map_err(|err| err.into())
    }
    .boxed()
  }

  fn handle_raw_unsubscribe_cmd(&self, endpoint: Endpoint) -> ButtplugServerResultFuture {
    let fut = self.hardware.unsubscribe(&HardwareUnsubscribeCmd::new(
      GENERIC_RAW_COMMAND_UUID,
      endpoint,
    ));
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if !raw_endpoints.contains(&endpoint) {
        return Ok(message::OkV0::new(1).into());
      }
      let result = fut
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|err| err.into());
      raw_endpoints.remove(&endpoint);
      result
    }
    .boxed()
  }

  fn handle_raw_subscribe_cmd(&self, endpoint: Endpoint) -> ButtplugServerResultFuture {
    let fut = self.hardware.subscribe(&HardwareSubscribeCmd::new(
      GENERIC_RAW_COMMAND_UUID,
      endpoint,
    ));
    let raw_endpoints = self.raw_subscribed_endpoints.clone();
    async move {
      if raw_endpoints.contains(&endpoint) {
        return Ok(message::OkV0::new(1).into());
      }
      let result = fut
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|err| err.into());
      raw_endpoints.insert(endpoint);
      result
    }
    .boxed()
  }
}
