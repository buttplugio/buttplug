// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
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

use buttplug_core::util::{async_manager, task::TaskScope};
use futures::future;
use tokio::{select, sync::mpsc::Receiver, time::Instant};
use tokio_util::sync::CancellationToken;

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

/// A batch of hardware commands for the device io task, optionally carrying a
/// write acknowledgement channel. When `ack` is present the batch is urgent:
/// the io task flushes it (and any pending batched commands) to hardware
/// immediately, then fires the ack.
pub struct DeviceTaskMessage {
  pub commands: Vec<HardwareCommand>,
  pub ack: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Spawn the device communication task.
///
/// This task handles:
/// - Receiving hardware commands from the internal channel
/// - Batching and deduplicating commands when message_gap is set
/// - Sending keepalive packets to maintain device connection
/// - Detecting hardware disconnection
///
/// Returns immediately after spawning the task.
pub fn spawn_device_task(
  task_scope: &TaskScope,
  hardware: Arc<Hardware>,
  _handler: Arc<dyn ProtocolHandler>,
  config: DeviceTaskConfig,
  mut command_receiver: Receiver<DeviceTaskMessage>,
) {
  task_scope.spawn("io", move |token| async move {
    run_device_task(hardware, config, &mut command_receiver, token).await;
  });
}

/// Run the device communication task (internal implementation).
///
/// This is separated from spawn_device_task to allow for easier testing
/// and potential future use in non-spawned contexts.
async fn run_device_task(
  hardware: Arc<Hardware>,
  config: DeviceTaskConfig,
  command_receiver: &mut Receiver<DeviceTaskMessage>,
  token: CancellationToken,
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

  // Write every pending command to hardware in order, updating the keepalive
  // packet as writes go out. Shared by the receive arm (urgent acked flush),
  // the batch-deadline arm, and the flush-on-exit hardening below.
  async fn flush_pending(
    hardware: &Hardware,
    pending_commands: &mut VecDeque<HardwareCommand>,
    track_keepalive: bool,
    keepalive_packet: &mut Option<HardwareWriteCmd>,
  ) {
    while let Some(cmd) = pending_commands.pop_front() {
      let _ = hardware.parse_message(&cmd).await;
      if track_keepalive && let HardwareCommand::Write(ref write_cmd) = cmd {
        *keepalive_packet = Some(write_cmd.clone());
      }
    }
  }

  loop {
    // Calculate keepalive timeout
    let keepalive_fut = async {
      if let Some(duration) = strategy_duration {
        async_manager::sleep(duration).await;
      } else if requires_keepalive {
        async_manager::sleep(Duration::from_secs(5)).await; // iOS Bluetooth default
      } else {
        future::pending::<()>().await;
      }
    };

    // Calculate batch flush timeout (only if we're batching)
    let batch_fut = async {
      match batch_deadline {
        Some(deadline) => {
          async_manager::sleep(deadline.saturating_duration_since(Instant::now())).await
        }
        None => future::pending::<()>().await,
      }
    };

    select! {
      biased;

      // Priority 0: Cooperative cancellation - wins over new work.
      _ = token.cancelled() => {
        info!("Device task cancelled, shutting down");
        // Best-effort flush of any batched writes (e.g. a stop command still
        // sitting in the batch window) before exiting. The hardware is still
        // present here, so the writes can land; write errors are swallowed.
        flush_pending(
          &hardware,
          &mut pending_commands,
          track_keepalive,
          &mut keepalive_packet,
        )
        .await;
        return;
      }

      // Priority 1: Incoming commands
      msg = command_receiver.recv() => {
        let Some(DeviceTaskMessage { commands, ack }) = msg else {
          info!("No longer receiving messages from device parent, breaking");
          // The command channel closed (all DeviceHandles dropped). Hardware is
          // still present, so best-effort flush any batched writes before exit.
          flush_pending(
            &hardware,
            &mut pending_commands,
            track_keepalive,
            &mut keepalive_packet,
          )
          .await;
          break;
        };

        if let Some(ack) = ack {
          // Urgent write-acknowledged batch (stop path). Merge with any pending
          // batch using the existing dedupe, flush everything to hardware now
          // regardless of the batch deadline, then fire the ack so the caller
          // only resolves once the writes have actually gone out.
          for command in commands {
            pending_commands.retain(|existing| !command.overlaps(existing));
            pending_commands.push_back(command);
          }
          flush_pending(
            &hardware,
            &mut pending_commands,
            track_keepalive,
            &mut keepalive_packet,
          )
          .await;
          batch_deadline = None;
          let _ = ack.send(());
        } else if let Some(device_wait_duration) = device_wait_duration {
          // Batching enabled
          if pending_commands.is_empty() {
            // First batch - add directly without deduplication (matches old behavior)
            pending_commands.extend(commands);
            batch_deadline = Some(Instant::now() + device_wait_duration);
          } else {
            // Subsequent batches - deduplicate each command against existing
            for command in commands {
              pending_commands.retain(|existing| !command.overlaps(existing));
              pending_commands.push_back(command);
            }
          }
        } else {
          // No batching - send immediately
          trace!("No wait duration, sending commands immediately: {:?}", commands);
          pending_commands.extend(commands);
          flush_pending(
            &hardware,
            &mut pending_commands,
            track_keepalive,
            &mut keepalive_packet,
          )
          .await;
        }
      }

      // Priority 2: Batch deadline reached - flush pending commands
      _ = batch_fut => {
        trace!("Batch deadline reached, sending {} commands", pending_commands.len());
        flush_pending(
          &hardware,
          &mut pending_commands,
          track_keepalive,
          &mut keepalive_packet,
        )
        .await;
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
          // Do NOT flush pending_commands here: the hardware is gone, so writes
          // would fail and any pending stop is moot.
          return;
        }
      }
    }
  }
  info!("Leaving task for {}", hardware.name());
}
