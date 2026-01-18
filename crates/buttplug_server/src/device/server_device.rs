// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Server Device Building
//!
//! This module provides the `build_device_handle` function which constructs a DeviceHandle
//! from hardware connectors and protocol specializers.

use std::{sync::Arc, time::Duration};

use buttplug_core::{
  errors::ButtplugDeviceError,
  message::{self, OutputType, OutputValue, StopDeviceCmdV4},
};
use buttplug_server_device_config::DeviceConfigurationManager;
use tokio::sync::mpsc::channel;

use crate::{
  device::{
    DeviceHandle,
    device_task::{spawn_device_task, DeviceTaskConfig},
    hardware::{HardwareCommand, HardwareConnector},
    protocol::{ProtocolKeepaliveStrategy, ProtocolSpecializer},
  },
  message::{
    checked_output_cmd::CheckedOutputCmdV4,
    spec_enums::ButtplugDeviceCommandMessageUnionV4,
  },
};

/// Build a DeviceHandle from hardware connectors and protocol specializers.
///
/// This function:
/// 1. Connects to the hardware
/// 2. Specializes it for the matched protocol
/// 3. Initializes the protocol handler
/// 4. Spawns the device communication task
/// 5. Returns a DeviceHandle for interacting with the device
pub(super) async fn build_device_handle(
  device_config_manager: Arc<DeviceConfigurationManager>,
  mut hardware_connector: Box<dyn HardwareConnector>,
  protocol_specializers: Vec<ProtocolSpecializer>,
) -> Result<DeviceHandle, ButtplugDeviceError> {
  // At this point, we know we've got hardware that is waiting to connect, and enough protocol
  // info to actually do something after we connect. So go ahead and connect.
  trace!("Connecting to {:?}", hardware_connector);
  let mut hardware_specializer = hardware_connector.connect().await?;

  // We can't run these in parallel because we need to only accept one specializer.
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

  let (identifier, mut protocol_initializer) = protocol_identifier_stage
    .identify(hardware.clone(), hardware_connector.specifier())
    .await?;

  // Now we have an identifier. After this point, if anything fails, consider it a complete
  // connection failure, as identify may have already run commands on the device, and therefore
  // put it in an unknown state if anything fails.

  // Check in the DeviceConfigurationManager to make sure we have attributes for this device.
  let definition = if let Some(attrs) = device_config_manager.device_definition(&identifier) {
    attrs
  } else {
    return Err(ButtplugDeviceError::DeviceConfigurationError(format!(
      "No protocols with viable protocol attributes for hardware {identifier:?}."
    )));
  };

  // Build the protocol handler
  let handler = protocol_initializer
    .initialize(hardware.clone(), &definition)
    .await?;

  let requires_keepalive = hardware.requires_keepalive();
  let strategy = handler.keepalive_strategy();

  // Create the hardware command channel and spawn the device task
  let (internal_hw_msg_sender, internal_hw_msg_recv) = channel::<Vec<HardwareCommand>>(1024);

  let device_wait_duration = if let Some(gap) = definition.message_gap_ms() {
    Some(Duration::from_millis(gap as u64))
  } else {
    hardware.message_gap()
  };

  spawn_device_task(
    hardware.clone(),
    handler.clone(),
    DeviceTaskConfig {
      message_gap: device_wait_duration,
      requires_keepalive: hardware.requires_keepalive(),
      keepalive_strategy: handler.keepalive_strategy(),
    },
    internal_hw_msg_recv,
  );

  // Generate stop commands for this device
  let mut stop_commands: Vec<ButtplugDeviceCommandMessageUnionV4> = vec![];
  for feature in definition.features().values() {
    if let Some(output_map) = feature.output() {
      for actuator_type in output_map.output_types() {
        let mut stop_cmd = |actuator_cmd| {
          stop_commands.push(
            CheckedOutputCmdV4::new(1, 0, feature.index(), feature.id(), actuator_cmd).into(),
          );
        };

        // Break out of these if one is found, we only need 1 stop message per output.
        match actuator_type {
          OutputType::Constrict => {
            stop_cmd(message::OutputCommand::Constrict(OutputValue::new(0)));
            break;
          }
          OutputType::Temperature => {
            stop_cmd(message::OutputCommand::Temperature(OutputValue::new(0)));
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

  // Create the DeviceHandle
  let device_handle = DeviceHandle::new(
    hardware,
    handler,
    definition.clone(),
    identifier,
    stop_commands,
    internal_hw_msg_sender,
  );

  // If we need a keepalive with a packet replay, set this up via stopping the device on connect.
  if ((requires_keepalive
    && matches!(
      strategy,
      ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
    ))
    || matches!(
      strategy,
      ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
    ))
    && let Err(e) = device_handle
      .parse_message(StopDeviceCmdV4::new(0, true, true).into())
      .await
  {
    return Err(ButtplugDeviceError::DeviceConnectionError(format!(
      "Error setting up keepalive: {e}"
    )));
  }

  Ok(device_handle)
}
