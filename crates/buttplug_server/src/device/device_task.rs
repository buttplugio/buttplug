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

use buttplug_core::util::async_manager;
use futures::future;
use tokio::{
  select,
  sync::{mpsc::Receiver, oneshot},
  time::Instant,
};

use super::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy},
};

/// Bounded wait for a write-acknowledged command to reach hardware.
///
/// Stop commands (and therefore shutdown) resolve Ok after this elapses even if
/// the device io task never reports the write, so a wedged or dead device cannot
/// hang shutdown.
pub(crate) const WRITE_ACK_TIMEOUT: Duration = Duration::from_secs(1);

/// A unit of work handed to the device io task.
///
/// A message carrying an [`oneshot::Sender`] ack is *urgent*: the io task merges
/// it into any pending batch, flushes everything to hardware immediately
/// regardless of the batch deadline, then fires the ack. Messages without an ack
/// keep the exact prior batching behaviour, so normal output is unchanged.
pub struct DeviceTaskMessage {
  /// Hardware commands to write.
  pub commands: Vec<HardwareCommand>,
  /// When set, the io task flushes immediately and signals once the write has
  /// reached hardware.
  pub write_ack: Option<oneshot::Sender<()>>,
}

impl DeviceTaskMessage {
  /// Build a fire-and-forget message (normal output path).
  pub fn fire_and_forget(commands: Vec<HardwareCommand>) -> Self {
    Self {
      commands,
      write_ack: None,
    }
  }

  /// Build a write-acknowledged message and return the receiver its caller
  /// awaits to know the write reached hardware.
  pub fn acknowledged(commands: Vec<HardwareCommand>) -> (Self, oneshot::Receiver<()>) {
    let (tx, rx) = oneshot::channel();
    (
      Self {
        commands,
        write_ack: Some(tx),
      },
      rx,
    )
  }
}

/// Configuration for the device task
pub struct DeviceTaskConfig {
  /// Duration to wait before flushing batched commands (None = no batching)
  pub message_gap: Option<Duration>,
  /// Whether the hardware requires keepalive packets
  pub requires_keepalive: bool,
  /// The keepalive strategy from the protocol handler
  pub keepalive_strategy: ProtocolKeepaliveStrategy,
}

/// Run the device communication task under its device owner's task group.
pub async fn run_owned_device_task(
  hardware: Arc<Hardware>,
  _handler: Arc<dyn ProtocolHandler>,
  config: DeviceTaskConfig,
  mut command_receiver: Receiver<DeviceTaskMessage>,
) {
  run_device_task(hardware, config, &mut command_receiver).await;
}

/// Run the device communication task (internal implementation).
///
/// This is separated from spawn_device_task to allow for easier testing
/// and potential future use in non-spawned contexts.
/// Drain every pending command to hardware, returning the last write command so
/// the caller can record it for keepalive replay. Shared by the batch-deadline
/// and urgent-flush paths so neither duplicates the flush logic.
async fn flush_pending(
  hardware: &Hardware,
  pending: &mut VecDeque<HardwareCommand>,
  track_keepalive: bool,
) -> Option<HardwareWriteCmd> {
  let mut last_write: Option<HardwareWriteCmd> = None;
  while let Some(cmd) = pending.pop_front() {
    let _ = hardware.parse_message(&cmd).await;
    if track_keepalive && let HardwareCommand::Write(ref write_cmd) = cmd {
      last_write = Some(write_cmd.clone());
    }
  }
  last_write
}

async fn run_device_task(
  hardware: Arc<Hardware>,
  config: DeviceTaskConfig,
  command_receiver: &mut Receiver<DeviceTaskMessage>,
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

      // Priority 1: Incoming commands
      msg = command_receiver.recv() => {
        let Some(message) = msg else {
          info!("No longer receiving messages from device parent, breaking");
          // Best-effort flush so a stop sitting in the batch window still lands
          // when our command channel closes (e.g. during shutdown teardown).
          // We are about to break, so keepalive tracking is unnecessary here.
          let _ = flush_pending(&hardware, &mut pending_commands, false).await;
          break;
        };
        let commands = message.commands;
        let write_ack = message.write_ack;

        if let Some(ack) = write_ack {
          // An acknowledged message is urgent (stop path): merge it into any
          // pending batch with the standard dedupe, flush everything to hardware
          // now regardless of the batch deadline, then signal the caller. The
          // dedupe must apply even with an empty pending queue: a multi-feature
          // stop accumulates one full-state write per feature in a single
          // message, and only the final state may reach hardware.
          for command in commands {
            pending_commands.retain(|existing| !command.overlaps(existing));
            pending_commands.push_back(command);
          }
          if let Some(write) =
            flush_pending(&hardware, &mut pending_commands, track_keepalive).await
          {
            keepalive_packet = Some(write);
          }
          batch_deadline = None;
          // Acknowledgement is best-effort: a dropped receiver means the caller
          // no longer cares (e.g. they raced ahead to disconnect).
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
          for cmd in commands {
            let _ = hardware.parse_message(&cmd).await;
            if track_keepalive
              && let HardwareCommand::Write(ref write_cmd) = cmd
            {
              keepalive_packet = Some(write_cmd.clone());
            }
          }
          if let Some(ack) = write_ack {
            let _ = ack.send(());
          }
        }
      }

      // Priority 2: Batch deadline reached - flush pending commands
      _ = batch_fut => {
        trace!("Batch deadline reached, sending {} commands", pending_commands.len());
        if let Some(write) =
          flush_pending(&hardware, &mut pending_commands, track_keepalive).await
        {
          keepalive_packet = Some(write);
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::device::{
    hardware::{
      HardwareInternal,
      HardwareReadCmd,
      HardwareReading,
      HardwareSubscribeCmd,
      HardwareUnsubscribeCmd,
    },
    protocol::ProtocolKeepaliveStrategy,
  };
  use buttplug_server_device_config::Endpoint;
  use futures::future::{self, BoxFuture, FutureExt};
  use std::sync::Mutex as StdMutex;
  use tokio::sync::broadcast;
  use uuid::Uuid;

  struct RecordingHardwareInternal {
    writes: Arc<StdMutex<Vec<HardwareWriteCmd>>>,
    event_sender: broadcast::Sender<HardwareEvent>,
  }

  impl RecordingHardwareInternal {
    fn new(writes: Arc<StdMutex<Vec<HardwareWriteCmd>>>) -> Self {
      let (event_sender, _) = broadcast::channel(256);
      Self {
        writes,
        event_sender,
      }
    }
  }

  impl HardwareInternal for RecordingHardwareInternal {
    fn disconnect(
      &self,
    ) -> BoxFuture<'static, Result<(), buttplug_core::errors::ButtplugDeviceError>> {
      future::ready(Ok(())).boxed()
    }

    fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
      self.event_sender.subscribe()
    }

    fn read_value(
      &self,
      msg: &HardwareReadCmd,
    ) -> BoxFuture<'static, Result<HardwareReading, buttplug_core::errors::ButtplugDeviceError>>
    {
      future::ready(Ok(HardwareReading::new(msg.endpoint(), &[]))).boxed()
    }

    fn write_value(
      &self,
      msg: &HardwareWriteCmd,
    ) -> BoxFuture<'static, Result<(), buttplug_core::errors::ButtplugDeviceError>> {
      self
        .writes
        .lock()
        .expect("Test mutex poisoned")
        .push(msg.clone());
      future::ready(Ok(())).boxed()
    }

    fn subscribe(
      &self,
      _msg: &HardwareSubscribeCmd,
    ) -> BoxFuture<'static, Result<(), buttplug_core::errors::ButtplugDeviceError>> {
      future::ready(Ok(())).boxed()
    }

    fn unsubscribe(
      &self,
      _msg: &HardwareUnsubscribeCmd,
    ) -> BoxFuture<'static, Result<(), buttplug_core::errors::ButtplugDeviceError>> {
      future::ready(Ok(())).boxed()
    }
  }

  fn rotate_direction_toggle_write() -> HardwareCommand {
    HardwareWriteCmd::new(
      &[Uuid::new_v4()],
      Endpoint::Tx,
      b"RotateChange;".to_vec(),
      false,
    )
    .into()
  }

  fn rotate_speed_write(speed: u32) -> HardwareCommand {
    const SPEED_COMMAND_UUID: Uuid = uuid::uuid!("11111111-1111-1111-1111-111111111111");
    HardwareWriteCmd::new(
      &[SPEED_COMMAND_UUID],
      Endpoint::Tx,
      format!("Rotate:{speed};").into_bytes(),
      false,
    )
    .into()
  }

  #[tokio::test]
  async fn batched_direction_toggles_all_survive() {
    let writes = Arc::new(StdMutex::new(Vec::new()));
    let hardware = Arc::new(Hardware::new(
      "test",
      "test-address",
      &[Endpoint::Tx],
      &None,
      false,
      Box::new(RecordingHardwareInternal::new(writes.clone())),
    ));

    let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
    let config = DeviceTaskConfig {
      message_gap: Some(Duration::from_millis(75)),
      requires_keepalive: false,
      keepalive_strategy: ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy,
    };
    let task = tokio::spawn(async move {
      run_device_task(hardware, config, &mut receiver).await;
    });

    // Both messages land inside the 75ms batching window.
    sender
      .send(DeviceTaskMessage::fire_and_forget(vec![
        rotate_direction_toggle_write(),
        rotate_speed_write(10),
      ]))
      .await
      .unwrap();
    sender
      .send(DeviceTaskMessage::fire_and_forget(vec![
        rotate_direction_toggle_write(),
        rotate_speed_write(10),
      ]))
      .await
      .unwrap();

    async_manager::sleep(Duration::from_millis(150)).await;
    drop(sender);
    let _ = task.await;

    let recorded = writes.lock().expect("Test mutex poisoned");
    let toggle_count = recorded
      .iter()
      .filter(|cmd| cmd.data() == b"RotateChange;")
      .count();
    assert_eq!(
      toggle_count, 2,
      "Both direction toggles must reach hardware"
    );
  }
}
