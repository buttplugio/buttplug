// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2024 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Device Task - Unified async task for device communication
//!
//! This module contains the main event loop that handles:
//! - Outgoing hardware commands (with optional batching/deduplication)
//! - Keepalive packet management
//! - Hardware disconnect detection

use std::{collections::VecDeque, sync::Arc, time::Duration};

use buttplug_core::{message::{StopCmdV4}, util::{self, async_manager}};
use futures::future;
use tokio::{
  select,
  sync::mpsc::Receiver,
  time::Instant,
};

use crate::message::{checked_input_cmd::CheckedInputCmdV4, checked_output_cmd::CheckedOutputCmdV4};

use super::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};

/// Configuration for the device task
pub struct DeviceTaskConfig {
  /// Duration to wait before flushing batched commands (None = no batching)
  pub message_gap: Option<Duration>,
  /// Whether the hardware requires keepalive packets
  pub requires_keepalive: bool,
  /// The keepalive strategy from the protocol handler
  pub keepalive_strategy: ProtocolKeepaliveStrategy,
}

pub enum DeviceTaskMessage {
  OutputCmd(CheckedOutputCmdV4),
  InputCmd(CheckedInputCmdV4),
  StopCmd(StopCmdV4),
}

/// Spawn the device communication task.
///
/// This task handles:
/// - Receiving Buttplug Messages from the internal channel and turning them into hardware commands
/// - Batching and deduplicating commands when message_gap is set
/// - Sending keepalive packets to maintain device connection
/// - Detecting hardware disconnection
///
/// Returns immediately after spawning the task.
pub fn spawn_device_task(
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  config: DeviceTaskConfig,
  mut command_receiver: Receiver<Vec<HardwareCommand>>,
) {
  async_manager::spawn(async move {
    run_device_task(hardware, handler, config, &mut command_receiver).await;
  });
}

/// Run the device communication task (internal implementation).
///
/// This is separated from spawn_device_task to allow for easier testing
/// and potential future use in non-spawned contexts.
async fn run_device_task(
  hardware: Arc<Hardware>,
  handler: Arc<dyn ProtocolHandler>,
  config: DeviceTaskConfig,
  command_receiver: &mut Receiver<Vec<HardwareCommand>>,
) {
  let mut hardware_events = hardware.event_stream();
  let device_wait_duration = config.message_gap;
  let requires_keepalive = config.requires_keepalive;
  let strategy = config.keepalive_strategy;

  let strategy_duration =
    if let ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) = strategy {
      Some(duration)
    } else {
      None
    };

  // Track last write command for keepalive replay
  let track_keepalive = (requires_keepalive
    && matches!(
      strategy,
      ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
    ))
    || matches!(
      strategy,
      ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(_)
    );
  let mut keepalive_packet: Option<HardwareWriteCmd> = None;

  // Batching state: pending commands and when to flush them
  let mut pending_commands: VecDeque<HardwareCommand> = VecDeque::new();
  let mut batch_deadline: Option<Instant> = None;

  loop {
    // Calculate keepalive timeout
    let keepalive_fut = async {
      if let Some(duration) = strategy_duration {
        util::sleep(duration).await;
      } else if requires_keepalive {
        util::sleep(Duration::from_secs(5)).await; // iOS Bluetooth default
      } else {
        future::pending::<()>().await;
      }
    };

    // Calculate batch flush timeout (only if we're batching)
    let batch_fut = async {
      match batch_deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => future::pending::<()>().await,
      }
    };

    select! {
      biased;

      // Priority 1: Incoming commands
      msg = command_receiver.recv() => {
        let Some(commands) = msg else {
          info!("No longer receiving messages from device parent, breaking");
          break;
        };

        if device_wait_duration.is_none() {
          // No batching - send immediately
          trace!("No wait duration, sending commands immediately: {:?}", commands);
          for cmd in commands {
            let _ = hardware.parse_message(&cmd).await;
            if track_keepalive {
              if let HardwareCommand::Write(ref write_cmd) = cmd {
                keepalive_packet = Some(write_cmd.clone());
              }
            }
          }
        } else {
          // Batching enabled
          if pending_commands.is_empty() {
            // First batch - add directly without deduplication (matches old behavior)
            pending_commands.extend(commands);
            batch_deadline = Some(Instant::now() + device_wait_duration.unwrap());
          } else {
            // Subsequent batches - deduplicate each command against existing
            for command in commands {
              pending_commands.retain(|existing| !command.overlaps(existing));
              pending_commands.push_back(command);
            }
          }
        }
      }

      // Priority 2: Batch deadline reached - flush pending commands
      _ = batch_fut => {
        debug!("Batch deadline reached, sending {} commands", pending_commands.len());
        while let Some(cmd) = pending_commands.pop_front() {
          let _ = hardware.parse_message(&cmd).await;
          if track_keepalive {
            if let HardwareCommand::Write(ref write_cmd) = cmd {
              keepalive_packet = Some(write_cmd.clone());
            }
          }
        }
        batch_deadline = None;
      }

      // Priority 3: Keepalive timer
      _ = keepalive_fut => {
        let result = match &strategy {
          ProtocolKeepaliveStrategy::RepeatLastPacketStrategyWithTiming(duration) => {
            if hardware.time_since_last_write().await > *duration {
              if let Some(ref packet) = keepalive_packet {
                hardware.write_value(packet).await
              } else {
                warn!("No keepalive packet available, device may disconnect.");
                Ok(())
              }
            } else {
              Ok(())
            }
          }
          ProtocolKeepaliveStrategy::HardwareRequiredRepeatPacketStrategy(packet) => {
            hardware.write_value(packet).await
          }
          ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy => {
            if let Some(ref packet) = keepalive_packet {
              hardware.write_value(packet).await
            } else {
              Ok(())
            }
          }
        };
        if let Err(e) = result {
          warn!("Error writing keepalive packet: {:?}", e);
          break;
        }
      }

      // Priority 4: Hardware events (disconnection)
      hw_event = hardware_events.recv() => {
        if matches!(hw_event, Ok(HardwareEvent::Disconnected(_))) || hw_event.is_err() {
          info!("Hardware disconnected, shutting down task");
          return;
        }
      }
    }
  }
  info!("Leaving task for {}", hardware.name());
}
