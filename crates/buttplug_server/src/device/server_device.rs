// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Server Device Implementation
//!
//! This struct manages the trip from buttplug protocol actuator/sensor message to hardware
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

use buttplug_core::{
  ButtplugResultFuture,
  errors::{ButtplugDeviceError, ButtplugError},
  message::{
    self, ButtplugServerMessageV4, DeviceFeature, DeviceMessageInfoV4, InputCommandType, InputType,
    OutputRotateWithDirection, OutputType, OutputValue,
  },
  util::{self, async_manager, stream::convert_broadcast_receiver_to_stream},
};
use buttplug_server_device_config::{
  DeviceConfigurationManager, DeviceDefinition, UserDeviceIdentifier,
};

use crate::{
  ButtplugServerResultFuture,
  device::{
    hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent},
    protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, ProtocolSpecializer},
  },
  message::{
    ButtplugServerDeviceMessage, checked_input_cmd::CheckedInputCmdV4,
    checked_output_cmd::CheckedOutputCmdV4, server_device_attributes::ServerDeviceAttributes,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};
use core::hash::{Hash, Hasher};
use dashmap::DashMap;
use futures::future::{self, BoxFuture, FutureExt};
use getset::Getters;
use tokio::{
  select,
  sync::{
    Mutex,
    mpsc::{Sender, channel},
  },
  time::Instant,
};
use tokio_stream::StreamExt;
use uuid::Uuid;

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
  definition: DeviceDefinition,
  //output_command_manager: ActuatorCommandManager,
  /// Unique identifier for the device
  #[getset(get = "pub")]
  identifier: UserDeviceIdentifier,
  #[getset(get = "pub")]
  legacy_attributes: ServerDeviceAttributes,
  last_output_command: DashMap<Uuid, CheckedOutputCmdV4>,

  stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4>,
  internal_hw_msg_sender: Sender<Vec<HardwareCommand>>,
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

impl Eq for ServerDevice {}

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
    let attrs = if let Some(attrs) = device_config_manager.device_definition(&identifier) {
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
    if (requires_keepalive
      && matches!(
        strategy,
        ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
      ))
      || matches!(
        strategy,
        ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
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
    definition: &DeviceDefinition,
  ) -> Self {
    let (internal_hw_msg_sender, mut internal_hw_msg_recv) = channel::<Vec<HardwareCommand>>(1024);

    let device_wait_duration = if definition.message_gap().is_some() {
      definition.message_gap()
    } else {
      hardware.message_gap()
    };

    // Set up and start the packet send task
    {
      let hardware = hardware.clone();
      let strategy = handler.keepalive_strategy();
      let strategy_duration =
        if let ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) = strategy {
          Some(duration)
        } else {
          None
        };
      async_manager::spawn(async move {
        let mut hardware_events = hardware.event_stream();
        let keepalive_packet = Mutex::new(None);
        // TODO This needs to throw system error messages
        let send_hw_cmd = async |command| {
          let _ = hardware.parse_message(&command).await;
          if hardware.requires_keepalive()
            && matches!(
              strategy,
              ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
            )
          {
            if let HardwareCommand::Write(command) = command {
              *keepalive_packet.lock().await = Some(command);
            }
          };
        };
        loop {
          let requires_keepalive = hardware.requires_keepalive();
          let wait_duration_fut = async move {
            if let Some(duration) = strategy_duration {
              util::sleep(duration).await;
            } else if requires_keepalive {
              // This is really only for iOS Bluetooth
              util::sleep(Duration::from_secs(5)).await;
            } else {
              future::pending::<()>().await;
            };
          };
          select! {
            hw_event = hardware_events.recv() => {
              if let Ok(hw_event) = hw_event {
                if matches!(hw_event, HardwareEvent::Disconnected(_)) {
                  info!("Hardware disconnected, shutting down keepalive");
                  return;
                }
              } else {
                  info!("Hardware disconnected, shutting down keepalive");
                  return;
              }
            }
            msg = internal_hw_msg_recv.recv() => {
              if msg.is_none() {
                info!("No longer receiving message from device parent, breaking");
                break;
              }
              let hardware_cmd = msg.unwrap();
              if device_wait_duration.is_none() {
                trace!("No wait duration specified, sending hardware commands {:?}", hardware_cmd);
                // send and continue
                for cmd in hardware_cmd {
                  send_hw_cmd(cmd).await;
                }
                continue;
              }
              // Run commands in order, otherwise we may end up sending out of order. This may take a while,
              // but it's what 99% of protocols expect. If they want something else, they can implement it
              // themselves.
              //
              // If anything errors out, just bail on the command series. This most likely means the device
              // disconnected.
              let mut local_commands: VecDeque<HardwareCommand> = VecDeque::new();
              local_commands.append(&mut VecDeque::from(hardware_cmd));

              let sleep_until = Instant::now() + *device_wait_duration.as_ref().unwrap();
              loop {
                select! {
                  hw_event = hardware_events.recv() => {
                    if let Ok(hw_event) = hw_event {
                      if matches!(hw_event, HardwareEvent::Disconnected(_)) {
                        info!("Hardware disconnected, shutting down keepalive");
                        return;
                      }
                    } else {
                        info!("Hardware disconnected, shutting down keepalive");
                        return;
                    }
                  }
                  msg = internal_hw_msg_recv.recv() => {
                    if msg.is_none() {
                      info!("No longer receiving message from device parent, breaking");
                      local_commands.clear();
                      break;
                    }
                    // Run commands in order, otherwise we may end up sending out of order. This may take a while,
                    // but it's what 99% of protocols expect. If they want something else, they can implement it
                    // themselves.
                    //
                    // If anything errors out, just bail on the command series. This most likely means the device
                    // disconnected.
                    for command in msg.unwrap() {
                      local_commands.retain(|v| !command.overlaps(v));
                      local_commands.push_back(command);
                    }
                  }
                  _ = util::sleep(sleep_until - Instant::now()) => {
                    break;
                  }
                }
                if sleep_until < Instant::now() {
                  break;
                }
              }
              while let Some(command) = local_commands.pop_front() {
                debug!("Sending hardware command {:?}", command);
                send_hw_cmd(command).await;
              }
            }
            _ = wait_duration_fut => {
              let keepalive_packet = keepalive_packet.lock().await.clone();
              match &strategy {
                ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) => {
                  if hardware.time_since_last_write().await > *duration {
                    if let Some(packet) = keepalive_packet {
                      if let Err(e) = hardware.write_value(&packet).await {
                        warn!("Error writing keepalive packet: {:?}", e);
                        break;
                      }
                    }
                  }
                }
                ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(packet) => {
                  if let Err(e) = hardware.write_value(packet).await {
                    warn!("Error writing keepalive packet: {:?}", e);
                    break;
                  }
                }
                ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy => {
                  if let Some(packet) = keepalive_packet {
                    if let Err(e) = hardware.write_value(&packet).await {
                      warn!("Error writing keepalive packet: {:?}", e);
                      break;
                    }
                  }
                }
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
            stop_commands
              .push(CheckedOutputCmdV4::new(1, 0, index as u32, feature.id(), actuator_cmd).into());
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
            OutputType::Spray => {
              stop_cmd(message::OutputCommand::Spray(OutputValue::new(0)));
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
      // Generating legacy attributes is cheap, just do it right when we create the device.
      legacy_attributes: ServerDeviceAttributes::new(&definition.features()),
      last_output_command: DashMap::new(),
      stop_commands,
      internal_hw_msg_sender,
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
  pub fn event_stream(&self) -> impl futures::Stream<Item = ServerDeviceEvent> + Send + use<> {
    let identifier = self.identifier.clone();
    let hardware_stream = convert_broadcast_receiver_to_stream(self.hardware.event_stream())
      .filter_map(move |hardware_event| {
        let id = identifier.clone();
        match hardware_event {
          HardwareEvent::Disconnected(_) => Some(ServerDeviceEvent::Disconnected(id)),
          HardwareEvent::Notification(_address, _endpoint, _data) => {
            // TODO Does this still need to be here? Does this need to be routed to the protocol it's part of?
            None
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
      // Input messages
      ButtplugDeviceCommandMessageUnionV4::InputCmd(msg) => self.handle_input_cmd(msg),
      // Actuator messages
      ButtplugDeviceCommandMessageUnionV4::OutputCmd(msg) => self.handle_outputcmd_v4(&msg),
      ButtplugDeviceCommandMessageUnionV4::OutputVecCmd(msg) => {
        let mut futs = vec![];
        let msg_id = msg.id();
        for m in msg.value_vec() {
          futs.push(self.handle_outputcmd_v4(m))
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

  fn handle_outputcmd_v4(&self, msg: &CheckedOutputCmdV4) -> ButtplugServerResultFuture {
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
    let sender = self.internal_hw_msg_sender.clone();
    async move {
      let _ = sender.send(commands).await;
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

  fn handle_input_cmd(
    &self,
    message: CheckedInputCmdV4,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    match message.input_command() {
      InputCommandType::Read => self.handle_input_read_cmd(
        message.device_index(),
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Subscribe => self.handle_input_subscribe_cmd(
        message.device_index(),
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
      InputCommandType::Unsubscribe => self.handle_input_unsubscribe_cmd(
        message.feature_index(),
        message.feature_id(),
        message.input_type(),
      ),
    }
  }

  fn handle_input_read_cmd(
    &self,
    device_index: u32,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> BoxFuture<'static, Result<ButtplugServerMessageV4, ButtplugError>> {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_read_cmd(device_index, device, feature_index, feature_id, input_type)
        .await
        .map_err(|e| e.into())
        .map(|e| e.into())
    }
    .boxed()
  }

  fn handle_input_subscribe_cmd(
    &self,
    device_index: u32,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> ButtplugServerResultFuture {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_subscribe_cmd(device_index, device, feature_index, feature_id, input_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }

  fn handle_input_unsubscribe_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    input_type: InputType,
  ) -> ButtplugServerResultFuture {
    let device = self.hardware.clone();
    let handler = self.handler.clone();
    async move {
      handler
        .handle_input_unsubscribe_cmd(device, feature_index, feature_id, input_type)
        .await
        .map(|_| message::OkV0::new(1).into())
        .map_err(|e| e.into())
    }
    .boxed()
  }
}
