// Buttplug Rust Source Code File - See https://buttplug.io for more info.
// Licensed under the BSD 3-Clause license. See LICENSE in the project root.

//! Opt-in state scheduler: receive/coalesce while a single hardware write is in
//! flight. A stop is a barrier: newer motion cannot overwrite an unsent stop.
//! Never cancel a slow write and then submit another on the same connection;
//! after an error/timeout discard pending motion and disconnect instead.

use std::{collections::VecDeque, sync::Arc, time::Duration};

use buttplug_core::{errors::ButtplugDeviceError, util::async_manager};
use buttplug_server_device_config::Endpoint;
use futures::{FutureExt, future::BoxFuture};
use tokio::{
  select,
  sync::{mpsc::Receiver, oneshot},
  time::Instant,
};

use super::{
  device_task::{DeviceTaskConfig, DeviceTaskMessage},
  hardware::{Hardware, HardwareCommand, HardwareEvent},
  yiciyuan_motion::{MotionController, settings_for},
};

const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
const SLOW_WRITE: Duration = Duration::from_millis(200);

// Observability only: missing ticks never gate commands or trigger a reconnect.
#[derive(Default)]
struct TickMonitor {
  last_tick: Option<Instant>,
  samples: u64,
  max_gap: Duration,
}

impl TickMonitor {
  fn observe(&mut self, endpoint: Endpoint, data: &[u8], now: Instant) -> bool {
    if endpoint != Endpoint::RxBLEBattery || data.len() < 3 || data[..2] != [0x35, 0x14] {
      return false;
    }
    if let Some(last) = self.last_tick {
      self.max_gap = self.max_gap.max(now.duration_since(last));
    }
    self.last_tick = Some(now);
    self.samples += 1;
    true
  }

  fn age_ms(&self, now: Instant) -> Option<u128> {
    self
      .last_tick
      .map(|last| now.duration_since(last).as_millis())
  }
}

#[derive(Default)]
struct Batch {
  commands: VecDeque<HardwareCommand>,
  acks: Vec<oneshot::Sender<()>>,
  queued_at: Option<Instant>,
}

impl Batch {
  fn merge(&mut self, commands: impl IntoIterator<Item = HardwareCommand>) {
    for command in commands {
      self.queued_at.get_or_insert_with(Instant::now);
      self.commands.retain(|old| !command.overlaps(old));
      self.commands.push_back(command);
    }
  }
}

#[derive(Default)]
struct Pending {
  normal: Batch,
  urgent: Batch,
}

impl Pending {
  fn receive(&mut self, message: DeviceTaskMessage) {
    if let Some(ack) = message.write_ack {
      // Pending state before the stop can be replaced. State received AFTER
      // it remains in normal and must not erase the stop barrier.
      self.urgent.merge(self.normal.commands.drain(..));
      self.normal.queued_at = None;
      self.urgent.merge(message.commands);
      self.urgent.acks.push(ack);
    } else {
      self.normal.merge(message.commands);
    }
  }

  fn has_urgent(&self) -> bool {
    !self.urgent.acks.is_empty()
  }

  fn is_empty(&self) -> bool {
    !self.has_urgent() && self.normal.commands.is_empty()
  }

  fn take_next(&mut self) -> Batch {
    if self.has_urgent() {
      std::mem::take(&mut self.urgent)
    } else {
      std::mem::take(&mut self.normal)
    }
  }
}

async fn write_batch(
  hardware: Arc<Hardware>,
  mut batch: Batch,
  timeout: Duration,
) -> Result<(), ButtplugDeviceError> {
  let queued_ms = batch
    .queued_at
    .map(|t| t.elapsed().as_millis())
    .unwrap_or(0);
  debug!(
    "Latest-state output dispatch: queued_ms={queued_ms}, packets={}, urgent={}",
    batch.commands.len(),
    !batch.acks.is_empty()
  );
  while let Some(command) = batch.commands.pop_front() {
    let started = Instant::now();
    let result = select! {
      result = hardware.parse_message(&command) => result,
      _ = async_manager::sleep(timeout) => {
        Err(ButtplugDeviceError::DeviceCommunicationError(
          format!("Latest-state output write timed out after {} ms", timeout.as_millis())))
      }
    };
    let elapsed = started.elapsed();
    if elapsed >= SLOW_WRITE || result.is_err() {
      warn!(
        "Latest-state output write: elapsed_ms={}, success={}",
        elapsed.as_millis(),
        result.is_ok()
      );
    } else {
      debug!(
        "Latest-state output write: elapsed_ms={}",
        elapsed.as_millis()
      );
    }
    result?;
  }
  for ack in batch.acks {
    let _ = ack.send(());
  }
  Ok(())
}

pub(super) async fn run_latest_device_task(
  hardware: Arc<Hardware>,
  config: DeviceTaskConfig,
  command_receiver: Receiver<DeviceTaskMessage>,
) {
  let motion = (hardware.name() == "YCY-FJB-03")
    .then(|| MotionController::new(settings_for(hardware.address())));
  // FJB-03: deliberately ignore legacy saved 30/250ms gap overrides. Only
  // serialize writes and coalesce superseded state, with no artificial pacing.
  let gap = if motion.is_some() {
    Duration::ZERO
  } else {
    config.message_gap.unwrap_or(Duration::from_millis(75))
  };
  let updates = motion.as_ref().map(|_| super::yiciyuan_motion::subscribe());
  run(
    hardware,
    gap,
    command_receiver,
    WRITE_TIMEOUT,
    motion,
    updates,
  )
  .await;
}

async fn run(
  hardware: Arc<Hardware>,
  gap: Duration,
  mut receiver: Receiver<DeviceTaskMessage>,
  write_timeout: Duration,
  mut motion: Option<MotionController>,
  mut updates: Option<
    tokio::sync::watch::Receiver<
      std::collections::HashMap<String, super::yiciyuan_motion::MotionSettings>,
    >,
  >,
) {
  let mut events = hardware.event_stream();
  let mut pending = Pending::default();
  let mut flight: Option<BoxFuture<'static, Result<(), ButtplugDeviceError>>> = None;
  let mut next_write = Instant::now();
  let mut closed = false;
  let mut ticks = TickMonitor::default();
  let mut report_at = Instant::now() + Duration::from_secs(5);
  let mut last_input = Instant::now();
  let device_name = hardware.name().clone();
  info!(
    "Latest-state scheduler enabled: gap_ms={}, timeout_ms={}",
    gap.as_millis(),
    write_timeout.as_millis()
  );

  loop {
    if !closed && flight.is_none() {
      if let (Some(controller), Some(updates)) = (&mut motion, &mut updates) {
        let settings = updates
          .borrow_and_update()
          .get(hardware.address())
          .copied()
          .unwrap_or_default();
        controller.reconfigure(settings, Instant::now());
      }
    }
    if closed && flight.is_none() && pending.is_empty() {
      return;
    }
    let can_send = flight.is_none() && !pending.is_empty();
    let motion_deadline = motion.as_ref().and_then(|m| m.deadline());
    let delay = if pending.has_urgent() {
      Duration::ZERO
    } else {
      next_write.saturating_duration_since(Instant::now())
    };
    select! {
      biased;
      event = events.recv() => {
        match event {
          Ok(HardwareEvent::Disconnected(_)) | Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
          Ok(HardwareEvent::Notification(_, endpoint, data)) => {
            let previous = ticks.last_tick;
            if ticks.observe(endpoint, &data, Instant::now()) && previous.is_none() {
              info!("{device_name} first device tick received");
            }
            if endpoint == Endpoint::RxBLEBattery && data.len() >= 7 && data[..2] == [0x35, 0x10] {
              info!("{device_name} device-info reply: model={}, firmware={}, motor_modes={}/{}/{}",
                data[2], data[3], data[4], data[5], data[6]);
            }
          }
          _ => {} // Notifications/lag do not imply disconnection.
        }
      }
      _ = async_manager::sleep(report_at.saturating_duration_since(Instant::now())) => {
        let now = Instant::now();
        let age = ticks.age_ms(now);
        info!("{device_name} tick summary: samples_5s={}, max_gap_ms={}, last_tick_age_ms={:?}, input_idle_ms={}",
          ticks.samples, ticks.max_gap.as_millis(), age, now.duration_since(last_input).as_millis());
        if age.is_none_or(|age| age > 2000) {
          warn!("{device_name} device ticks missing/stale; observation only, no automatic disconnect");
        }
        ticks.samples = 0;
        ticks.max_gap = Duration::ZERO;
        report_at = now + Duration::from_secs(5);
      }
      result = async { flight.as_mut().unwrap().await }, if flight.is_some() => {
        flight = None;
        if let Err(error) = result {
          error!("Latest-state output failed; discarding pending motion and disconnecting: {error}");
          // Dropping the write future is not proof of OS-level cancellation.
          // Do not retry or replay motion on this connection.
          select! {
            _ = hardware.disconnect() => {},
            _ = async_manager::sleep(Duration::from_secs(2)) => {},
          }
          return;
        }
        if let Some(controller) = &mut motion { controller.written(Instant::now()); }
      }
      _ = async { updates.as_mut().unwrap().changed().await }, if !closed && flight.is_none() && updates.is_some() => {}
      // Timer precedes input so a continuous producer cannot starve output.
      _ = async_manager::sleep(delay), if can_send => {
        // Take a bounded snapshot of already queued messages before dispatch.
        // Never drain indefinitely under an unbounded continuous producer.
        let count = receiver.len();
        for _ in 0..count {
          if let Ok(message) = receiver.try_recv() {
            last_input = Instant::now();
            pending.receive(message);
          }
        }
        let mut batch = pending.take_next();
        if let Some(controller) = &mut motion {
          batch.commands = controller.transform(batch.commands, Instant::now());
        }
        debug!("{device_name} dispatch tick_age_ms={:?}", ticks.age_ms(Instant::now()));
        next_write = Instant::now() + gap;
        flight = Some(write_batch(hardware.clone(), batch, write_timeout).boxed());
      }
      message = receiver.recv(), if !closed => {
        match message {
          Some(message) => {
            last_input = Instant::now();
            pending.receive(message);
          }
          None => {
            closed = true;
            // Preserve the scheduler's no-replay policy for every opted-in model.
            pending = Pending::default();
            if let Some(controller) = &mut motion {
              // The input owner is gone: cancel timers, discard pending motion,
              // and make a best-effort zero batch after any in-flight write.
              let (stop, ack) = DeviceTaskMessage::acknowledged(controller.stop_all().into_iter().collect());
              drop(ack);
              pending.receive(stop);
            }
          },
        }
      }
      _ = async_manager::sleep(motion_deadline.map(|t| t.saturating_duration_since(Instant::now())).unwrap_or(Duration::from_secs(3600))),
        if !closed && flight.is_none() && pending.is_empty() && receiver.is_empty() && motion_deadline.is_some() => {
          let commands = motion.as_mut().unwrap().tick(Instant::now());
          if !commands.is_empty() {
            let batch = Batch { commands, acks: vec![], queued_at: Some(Instant::now()) };
            flight = Some(write_batch(hardware.clone(), batch, write_timeout).boxed());
          }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::device::hardware::{
    HardwareInternal,
    HardwareReadCmd,
    HardwareReading,
    HardwareSubscribeCmd,
    HardwareUnsubscribeCmd,
    HardwareWriteCmd,
  };
  use buttplug_server_device_config::Endpoint;
  use tokio::sync::{Semaphore, broadcast, mpsc};
  use uuid::Uuid;

  #[test]
  fn tick_monitor_filters_packets_and_measures_idle_gap() {
    let start = Instant::now();
    let mut ticks = TickMonitor::default();
    assert_eq!(ticks.age_ms(start), None);
    assert!(!ticks.observe(Endpoint::Tx, &[0x35, 0x14, 0], start));
    assert!(!ticks.observe(Endpoint::RxBLEBattery, &[0x35, 0x13, 1, 80], start));
    assert!(!ticks.observe(Endpoint::RxBLEBattery, &[0x35, 0x14], start));
    assert!(ticks.observe(Endpoint::RxBLEBattery, &[0x35, 0x14, 0], start));
    assert!(ticks.observe(
      Endpoint::RxBLEBattery,
      &[0x35, 0x14, 1],
      start + Duration::from_millis(100)
    ));
    assert_eq!(ticks.samples, 2);
    assert_eq!(ticks.max_gap, Duration::from_millis(100));
    assert_eq!(ticks.age_ms(start + Duration::from_secs(3)), Some(2900));
  }

  struct SlowHardware {
    events: broadcast::Sender<HardwareEvent>,
    starts: mpsc::UnboundedSender<u8>,
    permits: Arc<Semaphore>,
    fail: bool,
    report_a: bool,
  }

  impl HardwareInternal for SlowHardware {
    fn event_stream(&self) -> broadcast::Receiver<HardwareEvent> {
      self.events.subscribe()
    }
    fn disconnect(&self) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      let events = self.events.clone();
      async move {
        let _ = events.send(HardwareEvent::Disconnected("test".into()));
        Ok(())
      }
      .boxed()
    }
    fn write_value(
      &self,
      msg: &HardwareWriteCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      let starts = self.starts.clone();
      let value = if self.report_a && msg.data().len() == 6 {
        msg.data()[2]
      } else {
        msg.data()[0]
      };
      let permits = self.permits.clone();
      let fail = self.fail;
      async move {
        starts.send(value).unwrap();
        permits.acquire().await.unwrap().forget();
        if fail {
          Err(ButtplugDeviceError::DeviceCommunicationError(
            "test failure".into(),
          ))
        } else {
          Ok(())
        }
      }
      .boxed()
    }
    fn read_value(
      &self,
      msg: &HardwareReadCmd,
    ) -> BoxFuture<'static, Result<HardwareReading, ButtplugDeviceError>> {
      let endpoint = msg.endpoint();
      async move { Ok(HardwareReading::new(endpoint, &[])) }.boxed()
    }
    fn subscribe(
      &self,
      _: &HardwareSubscribeCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      async { Ok(()) }.boxed()
    }
    fn unsubscribe(
      &self,
      _: &HardwareUnsubscribeCmd,
    ) -> BoxFuture<'static, Result<(), ButtplugDeviceError>> {
      async { Ok(()) }.boxed()
    }
  }

  struct Harness {
    sender: mpsc::Sender<DeviceTaskMessage>,
    starts: mpsc::UnboundedReceiver<u8>,
    permits: Arc<Semaphore>,
    events: broadcast::Sender<HardwareEvent>,
    task: tokio::task::JoinHandle<()>,
  }

  impl Harness {
    fn new(gap: Duration, timeout: Duration, fail: bool) -> Self {
      Self::with_motion(gap, timeout, fail, None)
    }
    fn with_motion(
      gap: Duration,
      timeout: Duration,
      fail: bool,
      motion: Option<MotionController>,
    ) -> Self {
      Self::with_updates(gap, timeout, fail, motion, None)
    }
    fn with_updates(
      gap: Duration,
      timeout: Duration,
      fail: bool,
      motion: Option<MotionController>,
      updates: Option<
        tokio::sync::watch::Receiver<
          std::collections::HashMap<String, super::super::yiciyuan_motion::MotionSettings>,
        >,
      >,
    ) -> Self {
      let (events, _) = broadcast::channel(16);
      let (starts, receiver) = mpsc::unbounded_channel();
      let permits = Arc::new(Semaphore::new(0));
      let hardware = Arc::new(Hardware::new(
        "test",
        "test",
        &[Endpoint::Tx],
        &None,
        false,
        Box::new(SlowHardware {
          events: events.clone(),
          starts,
          permits: permits.clone(),
          fail,
          report_a: motion.is_some(),
        }),
      ));
      let (sender, commands) = mpsc::channel(32);
      let task = tokio::spawn(run(hardware, gap, commands, timeout, motion, updates));
      Self {
        sender,
        starts: receiver,
        permits,
        events,
        task,
      }
    }
    async fn send(&self, value: u8) {
      self
        .sender
        .send(DeviceTaskMessage::fire_and_forget(vec![command(value)]))
        .await
        .unwrap();
    }
    async fn started(&mut self) -> u8 {
      tokio::time::timeout(Duration::from_secs(1), self.starts.recv())
        .await
        .unwrap()
        .unwrap()
    }
    async fn finish(self) {
      let _ = self.events.send(HardwareEvent::Disconnected("test".into()));
      tokio::time::timeout(Duration::from_secs(1), self.task)
        .await
        .unwrap()
        .unwrap();
    }
  }

  fn command(value: u8) -> HardwareCommand {
    HardwareWriteCmd::new(&[Uuid::nil()], Endpoint::Tx, vec![value], false).into()
  }

  fn motion_command(value: u8) -> HardwareCommand {
    HardwareWriteCmd::new(
      &[Uuid::nil()],
      Endpoint::Tx,
      vec![0x35, 0x12, value, 0, 0, 0x47 + value],
      false,
    )
    .into()
  }
  fn motion_harness(timeout: Duration) -> Harness {
    use super::super::yiciyuan_motion::{MotionMode, MotionSettings};
    Harness::with_motion(
      Duration::ZERO,
      timeout,
      false,
      Some(MotionController::new(MotionSettings {
        mode: MotionMode::Alternate,
        dwell_ms: 500,
        pause_ms: 100,
      })),
    )
  }
  #[tokio::test]
  async fn live_mode_update_waits_for_inflight_write_and_stop_cancels_resume() {
    use super::super::yiciyuan_motion::{MotionMode, MotionSettings};
    let (settings, updates) = tokio::sync::watch::channel(std::collections::HashMap::new());
    let mut h = Harness::with_updates(
      Duration::ZERO,
      Duration::from_secs(2),
      false,
      Some(MotionController::new(MotionSettings::default())),
      Some(updates),
    );
    h.sender
      .send(DeviceTaskMessage::fire_and_forget(vec![motion_command(10)]))
      .await
      .unwrap();
    assert_eq!(h.started().await, 10);
    settings.send_replace(std::collections::HashMap::from([(
      "test".into(),
      MotionSettings {
        mode: MotionMode::Reverse,
        ..Default::default()
      },
    )]));
    assert!(
      tokio::time::timeout(Duration::from_millis(30), h.starts.recv())
        .await
        .is_err()
    );
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 30);
    settings.send_replace(std::collections::HashMap::new());
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    let (stop, ack) = DeviceTaskMessage::acknowledged(vec![motion_command(0)]);
    h.sender.send(stop).await.unwrap();
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    ack.await.unwrap();
    settings.send_replace(std::collections::HashMap::from([(
      "test".into(),
      MotionSettings {
        mode: MotionMode::Alternate,
        ..Default::default()
      },
    )]));
    assert!(
      tokio::time::timeout(Duration::from_millis(650), h.starts.recv())
        .await
        .is_err()
    );
    h.finish().await;
  }

  #[tokio::test]
  async fn motion_timer_runs_without_new_sender_updates_and_stops_on_zero() {
    let mut h = motion_harness(Duration::from_secs(2));
    h.sender
      .send(DeviceTaskMessage::fire_and_forget(vec![motion_command(10)]))
      .await
      .unwrap();
    assert_eq!(h.started().await, 10);
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 30);
    let (stop, ack) = DeviceTaskMessage::acknowledged(vec![motion_command(0)]);
    h.sender.send(stop).await.unwrap();
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    ack.await.unwrap();
    assert!(
      tokio::time::timeout(Duration::from_millis(650), h.starts.recv())
        .await
        .is_err()
    );
    h.finish().await;
  }
  #[tokio::test]
  async fn stop_during_direction_pause_never_replays_pending_reverse() {
    let mut h = motion_harness(Duration::from_secs(2));
    h.sender
      .send(DeviceTaskMessage::fire_and_forget(vec![motion_command(10)]))
      .await
      .unwrap();
    assert_eq!(h.started().await, 10);
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0); // pause write still in flight
    let (stop, ack) = DeviceTaskMessage::acknowledged(vec![motion_command(0)]);
    h.sender.send(stop).await.unwrap();
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    ack.await.unwrap();
    assert!(
      tokio::time::timeout(Duration::from_millis(650), h.starts.recv())
        .await
        .is_err()
    );
    h.finish().await;
  }
  #[tokio::test]
  async fn motion_write_timeout_ends_timer_without_resuming() {
    let mut h = motion_harness(Duration::from_millis(40));
    h.sender
      .send(DeviceTaskMessage::fire_and_forget(vec![motion_command(10)]))
      .await
      .unwrap();
    assert_eq!(h.started().await, 10);
    tokio::time::timeout(Duration::from_secs(1), &mut h.task)
      .await
      .unwrap()
      .unwrap();
    assert!(h.starts.try_recv().is_err());
  }

  #[tokio::test]
  async fn busy_write_keeps_only_latest_state() {
    let mut h = Harness::new(Duration::from_millis(10), Duration::from_secs(2), false);
    h.send(10).await;
    assert_eq!(h.started().await, 10);
    for value in 11..=100 {
      h.send(value).await;
    }
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 100);
    // No parallel writes while the last one is unresolved.
    assert!(
      tokio::time::timeout(Duration::from_millis(30), h.starts.recv())
        .await
        .is_err()
    );
    h.finish().await;
  }

  #[tokio::test]
  async fn stop_is_not_overwritten_by_new_motion() {
    let mut h = Harness::new(Duration::from_millis(10), Duration::from_secs(2), false);
    h.send(10).await;
    assert_eq!(h.started().await, 10);
    h.send(20).await;
    let (stop, ack) = DeviceTaskMessage::acknowledged(vec![command(0)]);
    h.sender.send(stop).await.unwrap();
    h.send(80).await;
    h.permits.add_permits(1);
    assert_eq!(h.started().await, 0);
    h.permits.add_permits(1);
    tokio::time::timeout(Duration::from_secs(1), ack)
      .await
      .unwrap()
      .unwrap();
    assert_eq!(h.started().await, 80);
    h.finish().await;
  }

  #[tokio::test]
  async fn first_command_and_stop_bypass_pacing_delay() {
    let mut h = Harness::new(Duration::from_secs(5), Duration::from_secs(2), false);
    h.send(20).await;
    assert_eq!(h.started().await, 20);
    h.permits.add_permits(1);
    let (stop, _ack) = DeviceTaskMessage::acknowledged(vec![command(0)]);
    h.sender.send(stop).await.unwrap();
    assert_eq!(h.started().await, 0);
    h.finish().await;
  }

  #[tokio::test]
  async fn timeout_disconnects_without_replaying_queued_motion() {
    let mut h = Harness::new(Duration::ZERO, Duration::from_millis(50), false);
    let mut events = h.events.subscribe();
    h.send(20).await;
    assert_eq!(h.started().await, 20);
    h.send(80).await;
    tokio::time::timeout(Duration::from_secs(1), &mut h.task)
      .await
      .unwrap()
      .unwrap();
    assert!(matches!(
      events.recv().await.unwrap(),
      HardwareEvent::Disconnected(_)
    ));
    assert!(h.starts.try_recv().is_err());
  }

  #[tokio::test]
  async fn failed_write_disconnects_without_replay() {
    let mut h = Harness::new(Duration::ZERO, Duration::from_secs(2), true);
    let mut events = h.events.subscribe();
    h.send(20).await;
    assert_eq!(h.started().await, 20);
    h.send(80).await;
    h.permits.add_permits(1);
    tokio::time::timeout(Duration::from_secs(1), &mut h.task)
      .await
      .unwrap()
      .unwrap();
    assert!(matches!(
      events.recv().await.unwrap(),
      HardwareEvent::Disconnected(_)
    ));
    assert!(h.starts.try_recv().is_err());
  }

  #[tokio::test]
  async fn disconnect_is_handled_while_write_is_blocked() {
    let mut h = Harness::new(Duration::ZERO, Duration::from_secs(2), false);
    h.send(20).await;
    assert_eq!(h.started().await, 20);
    h.send(80).await;
    h.finish().await;
  }
}
