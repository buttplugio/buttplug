// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Unified Device Task
//!
//! This module implements a single task per device that handles all device operations:
//! - Commands from DeviceHandle (output, input, stop, disconnect)
//! - Hardware events (disconnections, notifications)
//! - Protocol events (sensor readings, etc.)
//! - Keepalive packets (when required by hardware or protocol)
//! - Message timing/gap enforcement with command batching
//!
//! This replaces the previous multi-task architecture where each device could spawn 3-5+ tasks.

use std::{collections::VecDeque, sync::Arc, time::Duration};

use buttplug_core::{
  errors::{ButtplugDeviceError, ButtplugError},
  message::{self, InputType, OutputType, OutputValue},
  util::{self, async_manager},
};
use buttplug_server_device_config::{
  DeviceConfigurationManager, ServerDeviceDefinition, UserDeviceIdentifier,
};
use futures::StreamExt;
use tokio::{select, sync::mpsc, time::Instant};

use crate::{
  device::{
    hardware::{Hardware, HardwareCommand, HardwareConnector, HardwareEvent, HardwareWriteCmd},
    protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, ProtocolSpecializer},
  },
  message::{
    ButtplugServerDeviceMessage, checked_input_cmd::CheckedInputCmdV4,
    checked_output_cmd::CheckedOutputCmdV4,
  },
};

use super::device_handle::{DeviceCommand, DeviceHandle};

/// Events emitted by the device task
#[derive(Debug)]
pub enum DeviceTaskEvent {
  /// Device sent a notification (sensor reading, etc.)
  Notification(UserDeviceIdentifier, ButtplugServerDeviceMessage),
  /// Device disconnected
  Disconnected(UserDeviceIdentifier),
}

/// Configuration for the device task
pub struct DeviceTaskConfig {
  /// Hardware interface
  pub hardware: Arc<Hardware>,
  /// Protocol handler
  pub handler: Arc<dyn ProtocolHandler>,
  /// Device definition with features
  pub definition: ServerDeviceDefinition,
  /// Device identifier
  pub identifier: UserDeviceIdentifier,
  /// Channel to receive commands from DeviceHandle
  pub command_rx: mpsc::Receiver<DeviceCommand>,
  /// Channel to send events externally
  pub event_tx: mpsc::Sender<DeviceTaskEvent>,
}

/// Run the unified device task
///
/// This is the main entry point for device operation. It handles all device
/// communication in a single task using `tokio::select!`.
pub async fn run_device_task(config: DeviceTaskConfig) {
  let DeviceTaskConfig {
    hardware,
    handler,
    definition,
    identifier,
    mut command_rx,
    event_tx,
  } = config;

  // Calculate message gap duration
  let device_wait_duration = definition
    .message_gap_ms()
    .map(|gap| Duration::from_millis(gap as u64))
    .or_else(|| hardware.message_gap());

  // Keepalive configuration
  let requires_keepalive = hardware.requires_keepalive();
  let strategy = handler.keepalive_strategy();
  let strategy_duration = match &strategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) => Some(*duration),
    _ => None,
  };

  // Build stop commands for the device
  let stop_commands = build_stop_commands(&definition);

  // State for keepalive
  let mut keepalive_packet: Option<HardwareWriteCmd> = None;

  // State for last output commands (for deduplication at message level)
  let mut last_output_commands: std::collections::HashMap<uuid::Uuid, CheckedOutputCmdV4> =
    std::collections::HashMap::new();

  // State for hardware command batching during message gap
  let mut pending_hardware_commands: VecDeque<HardwareCommand> = VecDeque::new();

  // Hardware and protocol event streams
  let mut hardware_events = hardware.event_stream();
  let mut protocol_events = handler.event_stream();

  info!(
    "Starting unified device task for {} ({:?})",
    definition.name(),
    identifier
  );

  loop {
    // Calculate keepalive wait duration
    let keepalive_fut = async {
      if let Some(duration) = strategy_duration {
        util::sleep(duration).await;
      } else if requires_keepalive {
        // Default 5-second keepalive for iOS Bluetooth
        util::sleep(Duration::from_secs(5)).await;
      } else {
        futures::future::pending::<()>().await;
      }
    };

    select! {
      // Handle commands from DeviceHandle
      cmd = command_rx.recv() => {
        match cmd {
          Some(DeviceCommand::Output { cmd, response }) => {
            let result = handle_output_command(
              &cmd,
              &handler,
              &mut pending_hardware_commands,
              &mut last_output_commands,
            );

            // Send response immediately (fire and forget semantics like old code)
            let _ = response.send(result.map_err(ButtplugError::from));

            // If no message gap, send commands immediately
            if device_wait_duration.is_none() {
              if let Err(e) = send_pending_commands(
                &mut pending_hardware_commands,
                &hardware,
                &mut keepalive_packet,
                requires_keepalive,
                &strategy,
              ).await {
                warn!("Error sending commands: {:?}", e);
              }
            } else {
              // Batch commands during message gap window (inner loop like old code)
              let sleep_until = Instant::now() + device_wait_duration.unwrap();
              loop {
                select! {
                  biased;

                  inner_cmd = command_rx.recv() => {
                    match inner_cmd {
                      Some(DeviceCommand::Output { cmd: inner_out_cmd, response: inner_response }) => {
                        let inner_result = handle_output_command(
                          &inner_out_cmd,
                          &handler,
                          &mut pending_hardware_commands,
                          &mut last_output_commands,
                        );
                        let _ = inner_response.send(inner_result.map_err(ButtplugError::from));
                      }
                      Some(DeviceCommand::Stop { response: inner_response }) => {
                        // Stop breaks out of batching
                        let inner_result = handle_stop_command(
                          &stop_commands,
                          &handler,
                          &mut pending_hardware_commands,
                          &mut last_output_commands,
                        );
                        let _ = inner_response.send(inner_result.map_err(ButtplugError::from));
                        break;
                      }
                      Some(DeviceCommand::Input { cmd: inner_in_cmd, response: inner_response }) => {
                        let inner_result = handle_input_command(&inner_in_cmd, &handler, &hardware).await;
                        let _ = inner_response.send(inner_result);
                      }
                      Some(DeviceCommand::Disconnect) => {
                        info!("Disconnect requested during batching for {:?}", identifier);
                        let _ = hardware.disconnect().await;
                        return; // Exit entire task
                      }
                      None => {
                        info!("DeviceHandle dropped during batching for {:?}", identifier);
                        return; // Exit entire task
                      }
                    }
                  }

                  hw_event = hardware_events.recv() => {
                    if let Ok(HardwareEvent::Disconnected(_)) = hw_event {
                      info!("Hardware disconnected during batching: {:?}", identifier);
                      let _ = event_tx.send(DeviceTaskEvent::Disconnected(identifier.clone())).await;
                      return;
                    }
                  }

                  _ = tokio::time::sleep_until(sleep_until) => {
                    break;
                  }
                }
                // Also check if we've passed the deadline
                if Instant::now() >= sleep_until {
                  break;
                }
              }
              // Send all batched commands
              if let Err(e) = send_pending_commands(
                &mut pending_hardware_commands,
                &hardware,
                &mut keepalive_packet,
                requires_keepalive,
                &strategy,
              ).await {
                warn!("Error sending batched commands: {:?}", e);
              }
            }
          }
          Some(DeviceCommand::Input { cmd, response }) => {
            let result = handle_input_command(&cmd, &handler, &hardware).await;
            let _ = response.send(result);
          }
          Some(DeviceCommand::Stop { response }) => {
            let result = handle_stop_command(
              &stop_commands,
              &handler,
              &mut pending_hardware_commands,
              &mut last_output_commands,
            );
            // Stop commands should be sent immediately
            if let Err(e) = send_pending_commands(
              &mut pending_hardware_commands,
              &hardware,
              &mut keepalive_packet,
              requires_keepalive,
              &strategy,
            ).await {
              let _ = response.send(Err(e));
              continue;
            }

            // Unsubscribe from all inputs
            for (_i, f) in definition.features().iter() {
              if let Some(inputs) = f.input() {
                if inputs.can_subscribe() {
                  let _ = handler
                    .handle_input_unsubscribe_cmd(
                      hardware.clone(),
                      f.index(),
                      f.id(),
                      InputType::Unknown,
                    )
                    .await;
                }
              }
            }

            let _ = response.send(result.map_err(ButtplugError::from));
          }
          Some(DeviceCommand::Disconnect) => {
            info!("Disconnect requested for {:?}", identifier);
            let _ = hardware.disconnect().await;
            break;
          }
          None => {
            // Channel closed, DeviceHandle dropped
            info!("DeviceHandle dropped, shutting down task for {:?}", identifier);
            break;
          }
        }
      }

      // Handle hardware events
      hw_event = hardware_events.recv() => {
        match hw_event {
          Ok(HardwareEvent::Disconnected(_)) => {
            info!("Hardware disconnected: {:?}", identifier);
            let _ = event_tx.send(DeviceTaskEvent::Disconnected(identifier.clone())).await;
            break;
          }
          Ok(HardwareEvent::Notification(_address, _endpoint, _data)) => {
            // TODO: Route notifications to protocol handler if needed
          }
          Err(_) => {
            // Hardware event channel closed
            info!("Hardware event channel closed for {:?}", identifier);
            let _ = event_tx.send(DeviceTaskEvent::Disconnected(identifier.clone())).await;
            break;
          }
        }
      }

      // Handle protocol events
      proto_event = protocol_events.next() => {
        if let Some(message) = proto_event {
          let _ = event_tx.send(DeviceTaskEvent::Notification(identifier.clone(), message)).await;
        }
      }

      // Handle keepalive
      _ = keepalive_fut => {
        if let Err(e) = handle_keepalive(
          &hardware,
          &keepalive_packet,
          requires_keepalive,
          &strategy,
        ).await {
          warn!("Keepalive error for {:?}: {:?}", identifier, e);
          break;
        }
      }
    }
  }

  info!("Device task exiting for {:?}", identifier);
}

/// Build stop commands for all device features
fn build_stop_commands(definition: &ServerDeviceDefinition) -> Vec<CheckedOutputCmdV4> {
  let mut stop_commands = Vec::new();

  for feature in definition.features().values() {
    if let Some(output_map) = feature.output() {
      for actuator_type in output_map.output_types() {
        let stop_cmd = |actuator_cmd| {
          CheckedOutputCmdV4::new(1, 0, feature.index(), feature.id(), actuator_cmd)
        };

        // Only need one stop message per output
        match actuator_type {
          OutputType::Constrict => {
            stop_commands.push(stop_cmd(message::OutputCommand::Constrict(OutputValue::new(0))));
            break;
          }
          OutputType::Temperature => {
            stop_commands.push(stop_cmd(message::OutputCommand::Temperature(OutputValue::new(0))));
            break;
          }
          OutputType::Spray => {
            stop_commands.push(stop_cmd(message::OutputCommand::Spray(OutputValue::new(0))));
            break;
          }
          OutputType::Led => {
            stop_commands.push(stop_cmd(message::OutputCommand::Led(OutputValue::new(0))));
            break;
          }
          OutputType::Oscillate => {
            stop_commands.push(stop_cmd(message::OutputCommand::Oscillate(OutputValue::new(0))));
            break;
          }
          OutputType::Rotate => {
            stop_commands.push(stop_cmd(message::OutputCommand::Rotate(OutputValue::new(0))));
            break;
          }
          OutputType::Vibrate => {
            stop_commands.push(stop_cmd(message::OutputCommand::Vibrate(OutputValue::new(0))));
            break;
          }
          _ => {
            // Position commands don't have a meaningful "stop" value
            continue;
          }
        }
      }
    }
  }

  stop_commands
}

/// Queue hardware commands with overlap deduplication
///
/// This matches the old ServerDevice behavior: for each new command, remove any
/// existing commands that overlap with it, then add the new command.
fn queue_hardware_commands(
  commands: Vec<HardwareCommand>,
  pending_commands: &mut VecDeque<HardwareCommand>,
) {
  for command in commands {
    // Remove any existing commands that overlap with this one
    pending_commands.retain(|existing| !command.overlaps(existing));
    // Add the new command
    pending_commands.push_back(command);
  }
}

/// Handle an output command - queue hardware commands for batching
fn handle_output_command(
  cmd: &CheckedOutputCmdV4,
  handler: &Arc<dyn ProtocolHandler>,
  pending_commands: &mut VecDeque<HardwareCommand>,
  last_output_commands: &mut std::collections::HashMap<uuid::Uuid, CheckedOutputCmdV4>,
) -> Result<(), ButtplugDeviceError> {
  // Check for duplicate command
  if let Some(last_cmd) = last_output_commands.get(&cmd.feature_id()) {
    if last_cmd == cmd {
      trace!("Skipping duplicate output command");
      return Ok(());
    }
  }
  last_output_commands.insert(cmd.feature_id(), cmd.clone());

  // Get hardware commands from protocol handler
  let hardware_commands = handler.handle_output_cmd(cmd)?;

  // Queue hardware commands with overlap deduplication
  queue_hardware_commands(hardware_commands, pending_commands);

  Ok(())
}

/// Handle an input command
async fn handle_input_command(
  cmd: &CheckedInputCmdV4,
  handler: &Arc<dyn ProtocolHandler>,
  hardware: &Arc<Hardware>,
) -> Result<buttplug_core::message::ButtplugServerMessageV4, ButtplugError> {
  use buttplug_core::message::InputCommandType;

  match cmd.input_command() {
    InputCommandType::Read => {
      let reading = handler
        .handle_input_read_cmd(
          cmd.device_index(),
          hardware.clone(),
          cmd.feature_index(),
          cmd.feature_id(),
          cmd.input_type(),
        )
        .await
        .map_err(ButtplugError::from)?;
      Ok(reading.into())
    }
    InputCommandType::Subscribe => {
      handler
        .handle_input_subscribe_cmd(
          cmd.device_index(),
          hardware.clone(),
          cmd.feature_index(),
          cmd.feature_id(),
          cmd.input_type(),
        )
        .await
        .map_err(ButtplugError::from)?;
      Ok(message::OkV0::default().into())
    }
    InputCommandType::Unsubscribe => {
      handler
        .handle_input_unsubscribe_cmd(
          hardware.clone(),
          cmd.feature_index(),
          cmd.feature_id(),
          cmd.input_type(),
        )
        .await
        .map_err(ButtplugError::from)?;
      Ok(message::OkV0::default().into())
    }
  }
}

/// Handle a stop command - queue stop commands for batching
fn handle_stop_command(
  stop_commands: &[CheckedOutputCmdV4],
  handler: &Arc<dyn ProtocolHandler>,
  pending_commands: &mut VecDeque<HardwareCommand>,
  last_output_commands: &mut std::collections::HashMap<uuid::Uuid, CheckedOutputCmdV4>,
) -> Result<(), ButtplugDeviceError> {
  // Stop all outputs
  for cmd in stop_commands {
    // Clear from last commands so we don't skip duplicates
    last_output_commands.remove(&cmd.feature_id());

    let hardware_commands = handler.handle_output_cmd(cmd)?;

    // Queue hardware commands with overlap deduplication
    queue_hardware_commands(hardware_commands, pending_commands);
  }

  Ok(())
}

/// Send all pending hardware commands
async fn send_pending_commands(
  pending_commands: &mut VecDeque<HardwareCommand>,
  hardware: &Arc<Hardware>,
  keepalive_packet: &mut Option<HardwareWriteCmd>,
  requires_keepalive: bool,
  strategy: &ProtocolKeepaliveStrategy,
) -> Result<(), ButtplugError> {
  // Update keepalive packet tracking
  let should_track_keepalive = (requires_keepalive
    && matches!(
      strategy,
      ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
    ))
    || matches!(
      strategy,
      ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
    );

  while let Some(command) = pending_commands.pop_front() {
    // Track last write command for keepalive
    if should_track_keepalive {
      if let HardwareCommand::Write(ref write_cmd) = command {
        *keepalive_packet = Some(write_cmd.clone());
      }
    }

    // Send command
    debug!("Sending hardware command {:?}", command);
    hardware
      .parse_message(&command)
      .await
      .map_err(ButtplugError::from)?;
  }

  Ok(())
}

/// Handle keepalive packet sending
async fn handle_keepalive(
  hardware: &Arc<Hardware>,
  keepalive_packet: &Option<HardwareWriteCmd>,
  requires_keepalive: bool,
  strategy: &ProtocolKeepaliveStrategy,
) -> Result<(), ButtplugError> {
  match strategy {
    ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) => {
      if hardware.time_since_last_write().await > *duration {
        if let Some(packet) = keepalive_packet {
          hardware
            .write_value(packet)
            .await
            .map_err(ButtplugError::from)?;
        } else {
          warn!("No keepalive packet available, device may disconnect.");
        }
      }
    }
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(packet) => {
      hardware
        .write_value(packet)
        .await
        .map_err(ButtplugError::from)?;
    }
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy => {
      if requires_keepalive {
        if let Some(packet) = keepalive_packet {
          hardware
            .write_value(packet)
            .await
            .map_err(ButtplugError::from)?;
        }
      }
    }
  }
  Ok(())
}

/// Build a device and spawn the unified task
///
/// This function handles the entire device connection process:
/// 1. Connects to hardware via the connector
/// 2. Specializes hardware with protocol specifiers
/// 3. Identifies the device and gets configuration
/// 4. Initializes the protocol handler
/// 5. Creates a DeviceHandle and spawns the unified task
///
/// Returns the DeviceHandle and an event receiver for device events.
pub async fn build_device(
  device_config_manager: Arc<DeviceConfigurationManager>,
  mut hardware_connector: Box<dyn HardwareConnector>,
  protocol_specializers: Vec<ProtocolSpecializer>,
) -> Result<(DeviceHandle, mpsc::Receiver<DeviceTaskEvent>), ButtplugDeviceError> {
  // Connect to hardware
  trace!("Connecting to {:?}", hardware_connector);
  let mut hardware_specializer = hardware_connector.connect().await?;

  // Try each protocol specializer until one works
  let mut protocol_identifier = None;
  let mut hardware_out = None;
  for protocol_specializer in protocol_specializers {
    match hardware_specializer
      .specialize(protocol_specializer.specifiers())
      .await
    {
      Ok(specialized_hardware) => {
        protocol_identifier = Some(protocol_specializer.identify());
        hardware_out = Some(specialized_hardware);
        break;
      }
      Err(e) => {
        error!("{:?}", e.to_string());
      }
    }
  }

  if protocol_identifier.is_none() {
    return Err(ButtplugDeviceError::DeviceConfigurationError(
      "No protocols with viable communication matches for hardware.".to_owned(),
    ));
  }

  let mut protocol_identifier_stage = protocol_identifier.unwrap();
  let hardware = Arc::new(hardware_out.unwrap());

  // Identify the device
  let (identifier, mut protocol_initializer) = protocol_identifier_stage
    .identify(hardware.clone(), hardware_connector.specifier())
    .await?;

  // Get device configuration
  let definition = device_config_manager
    .device_definition(&identifier)
    .ok_or_else(|| {
      ButtplugDeviceError::DeviceConfigurationError(format!(
        "No protocols with viable protocol attributes for hardware {identifier:?}."
      ))
    })?;

  // Initialize the protocol handler
  let handler = protocol_initializer
    .initialize(hardware.clone(), &definition.clone())
    .await?;

  let requires_keepalive = hardware.requires_keepalive();
  let strategy = handler.keepalive_strategy();

  // Create channels for the device task
  let (command_tx, command_rx) = mpsc::channel::<DeviceCommand>(256);
  let (event_tx, event_rx) = mpsc::channel::<DeviceTaskEvent>(256);

  // Create the device handle
  let handle = DeviceHandle::new(command_tx, identifier.clone(), &definition)
    .map_err(|e| ButtplugDeviceError::DeviceConnectionError(format!("{e}")))?;

  // Create task configuration
  let task_config = DeviceTaskConfig {
    hardware: hardware.clone(),
    handler: handler.clone(),
    definition: definition.clone(),
    identifier: identifier.clone(),
    command_rx,
    event_tx: event_tx.clone(),
  };

  // Spawn the unified device task
  async_manager::spawn(async move {
    run_device_task(task_config).await;
  });

  // If we need a keepalive with packet replay, send initial stop command
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
    // Send stop command through the handle to initialize keepalive packet
    if let Err(e) = handle.stop().await {
      return Err(ButtplugDeviceError::DeviceConnectionError(format!(
        "Error setting up keepalive: {e}"
      )));
    }
  }

  Ok((handle, event_rx))
}
