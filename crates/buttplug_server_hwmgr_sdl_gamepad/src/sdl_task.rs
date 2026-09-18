// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Single SDL3 ownership thread for the SDL gamepad hardware manager.
//!
//! One dedicated thread owns the entire SDL3 context for the process and
//! multiplexes all gamepads. The thread never pumps SDL events (SDL3
//! documents `SDL_PumpEvents` as main-thread-only, and this manager does not
//! consume controller input): discovery is on-demand `SDL_GetGamepads`
//! enumeration, and removal detection is per-device connected-state polling.
//!
//! Gamepads are identified by SDL3 instance ID ([`JoystickId`]), which is
//! stable only for the lifetime of a connection. Conversion to buttplug's
//! string address space (`sdl-gamepad-{instance_id}`) happens only at the
//! communication-manager boundary; reconnects may receive a new instance ID
//! and reported name, so identity is connection-scoped.
//!
//! Rumble is armed with a finite duration (the sdl3 crate documents that
//! `u32::MAX` overflows and ends the effect immediately). Each active main or
//! trigger pair is refreshed independently by the thread before expiry, so
//! one-shot ScalarCmd commands hold indefinitely.

use sdl3::joystick::JoystickId;
use std::{
  collections::HashMap,
  sync::{Arc, OnceLock, mpsc},
  time::Duration,
};
use thiserror::Error;
use tokio::sync::{oneshot, watch};

/// Duration (ms) each rumble command is armed for. Finite on purpose: the sdl3
/// crate documents `u32::MAX` as overflowing and ending the effect immediately.
pub(crate) const RUMBLE_DURATION_MS: u32 = 60_000;

/// Interval (ms) at which a still-active (non-zero) rumble is re-armed. A
/// 100-millisecond keepalive, not a near-expiry refresh: on-hardware testing
/// showed Bluetooth controllers (DualSense, Joy-Con) stall effects between
/// one-second refreshes, so the current command is simply re-sent on every
/// loop wake while active (the loop already wakes at this cadence). The long
/// finite arm remains as a safety net if a keepalive is missed.
const RUMBLE_KEEPALIVE_INTERVAL_MS: u64 = 100;

/// Interval (ms) at which open gamepads have their connected state polled.
const CONNECTED_POLL_INTERVAL_MS: u64 = 500;

/// Timeout (ms) of the command-receive wait; also the loop's wake granularity
/// for connected-poll and rumble-refresh checks.
const COMMAND_WAKE_MS: u64 = 100;

/// Which independent rumble pairs an opened gamepad reported. Logical output
/// channels, not physical motor counts; can vary by OS/transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SdlRumbleCapabilities {
  pub rumble: bool,
  pub trigger_rumble: bool,
}

impl SdlRumbleCapabilities {
  pub fn any(self) -> bool {
    self.rumble || self.trigger_rumble
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SdlRumbleState {
  pub low: u16,
  pub high: u16,
  pub left_trigger: u16,
  pub right_trigger: u16,
}

impl SdlRumbleState {
  pub fn slots(&self) -> [u16; 4] {
    [self.low, self.high, self.left_trigger, self.right_trigger]
  }
}

/// A gamepad discovered by a scan, with its SDL-reported name (or the
/// deterministic fallback name when the name lookup failed).
#[derive(Debug, Clone)]
pub(crate) struct SdlGamepadDesc {
  pub id: JoystickId,
  pub name: String,
  pub capabilities: SdlRumbleCapabilities,
  pub is_open: bool,
}

/// Construct a [JoystickId] from its raw u32 value. `JoystickId` is a type
/// alias, so its constructor isn't reachable through the alias name.
#[cfg(test)]
pub(crate) fn joystick_id(n: u32) -> JoystickId {
  JoystickId::new(n)
}

#[derive(Debug, Error, Clone)]
pub(crate) enum SdlTaskError {
  #[error("SDL initialization failed: {0}")]
  Init(String),
  #[error("SDL gamepad scan failed: {0}")]
  Scan(String),
  #[error("SDL gamepad {0} has no rumble capability")]
  NoRumbleCapability(JoystickId),
  #[error("SDL gamepad {0} is already open")]
  AlreadyOpen(JoystickId),
  #[error("SDL gamepad {0} has been removed")]
  Removed(JoystickId),
  #[error("SDL gamepad open failed: {0}")]
  Open(String),
  #[error("SDL gamepad rumble failed: {0}")]
  Rumble(String),
  #[error("SDL gamepad thread is not running")]
  ThreadClosed,
}

#[derive(Debug, Error, Clone)]
#[error("SDL gamepad task failed to initialize: {0}")]
pub(crate) struct SdlTaskInitError(pub String);

/// Inner seam for the SDL3 calls used by the task.
///
/// Deliberately **not** `Send`: it is constructed, used, and dropped entirely
/// on the SDL thread (the sdl3 crate's `Sdl` type is `!Send`). Tests provide
/// fake implementations built from shared, `Send` state.
pub(crate) trait SdlDriver {
  fn enumerate(&mut self) -> Result<Vec<JoystickId>, String>;
  fn name_for_id(&mut self, id: JoystickId) -> Result<String, String>;
  fn open(&mut self, id: JoystickId) -> Result<Box<dyn DriverGamepad>, String>;
}

/// An opened gamepad on the SDL thread. Dropping closes it.
/// Transport of an opened gamepad, as far as SDL reports it. Used on macOS to
/// skip wired pads (see the scan handler for the rationale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DriverConnection {
  Wired,
  Wireless,
  Unknown,
}

pub(crate) trait DriverGamepad {
  fn has_rumble(&self) -> bool;
  fn has_rumble_triggers(&self) -> bool;
  fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String>;
  fn rumble_triggers(&mut self, left: u16, right: u16, duration_ms: u32) -> Result<(), String>;
  fn connected(&self) -> bool;
  /// Default `Unknown` so fakes only override it where relevant.
  fn connection_state(&self) -> DriverConnection {
    DriverConnection::Unknown
  }
}

/// Clock seam so rumble-refresh and poll timing are unit-testable. `Send`
/// because it moves into the SDL thread at spawn time.
pub(crate) trait SdlClock: Send {
  fn now_ms(&self) -> u64;
}

/// Production clock: monotonic milliseconds since SDL-thread start. Uses
/// `Instant` (not wall-clock `SystemTime`) so a backward clock adjustment can
/// never suppress rumble refresh long enough for the finite arm to lapse.
struct SystemClock {
  start: std::time::Instant,
}

impl SdlClock for SystemClock {
  fn now_ms(&self) -> u64 {
    self.start.elapsed().as_millis() as u64
  }
}

enum SdlCommand {
  Scan {
    reply: oneshot::Sender<Result<Vec<SdlGamepadDesc>, SdlTaskError>>,
  },
  Open {
    id: JoystickId,
    reply: oneshot::Sender<Result<(SdlOpenedGamepadHandle, SdlRumbleCapabilities), SdlTaskError>>,
  },
  #[cfg(test)]
  Shutdown { reply: oneshot::Sender<()> },
  SetRumbleState {
    id: JoystickId,
    generation: u64,
    state: SdlRumbleState,
    duration: u32,
    reply: oneshot::Sender<Result<(), SdlTaskError>>,
  },
  Close {
    id: JoystickId,
    generation: u64,
    reply: oneshot::Sender<()>,
  },
}

/// Handle to an opened gamepad, safe to use from async contexts on any thread.
///
/// Carries the open's `generation` so that a stale handle (e.g. a clone held
/// across a close/reopen of the same still-connected id) is inert: its rumble
/// commands fail with [`SdlTaskError::Removed`] and its closes are no-ops.
#[derive(Clone)]
pub(crate) struct SdlOpenedGamepadHandle {
  id: JoystickId,
  generation: u64,
  task: SdlTaskHandle,
  removed_rx: watch::Receiver<bool>,
}

impl std::fmt::Debug for SdlOpenedGamepadHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SdlOpenedGamepadHandle")
      .field("id", &self.id.0)
      .finish()
  }
}

impl SdlOpenedGamepadHandle {
  /// Receiver that yields `true` when the gamepad is closed or disconnected.
  pub(crate) fn removed(&self) -> watch::Receiver<bool> {
    self.removed_rx.clone()
  }

  pub(crate) async fn set_rumble_state(
    &self,
    state: SdlRumbleState,
    duration_ms: u32,
  ) -> Result<(), SdlTaskError> {
    self
      .task
      .set_rumble_state(self.id, self.generation, state, duration_ms)
      .await
  }

  pub(crate) async fn close(&self) -> Result<(), SdlTaskError> {
    self.task.close(self.id, self.generation).await
  }

  /// Fire-and-forget close usable from synchronous contexts (e.g. `Drop`).
  pub(crate) fn close_now(&self) {
    self.task.close_now(self.id, self.generation);
  }
}

/// Cloneable handle to the SDL thread's command channel.
///
/// The loop retains its own sender, so external handle drops do not stop it.
/// Production lives until process exit; tests explicitly use the Shutdown seam.
/// Channel disconnection is a defensive teardown path.
#[derive(Clone)]
pub(crate) struct SdlTaskHandle {
  cmd_tx: mpsc::Sender<SdlCommand>,
}

impl std::fmt::Debug for SdlTaskHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("SdlTaskHandle").finish()
  }
}

impl SdlTaskHandle {
  async fn send_and_await<R>(
    &self,
    make_cmd: impl FnOnce(oneshot::Sender<R>) -> SdlCommand,
  ) -> Result<R, SdlTaskError> {
    let (reply_tx, reply_rx) = oneshot::channel();
    self
      .cmd_tx
      .send(make_cmd(reply_tx))
      .map_err(|_| SdlTaskError::ThreadClosed)?;
    reply_rx.await.map_err(|_| SdlTaskError::ThreadClosed)
  }

  pub(crate) async fn scan(&self) -> Result<Vec<SdlGamepadDesc>, SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Scan { reply })
      .await?
  }

  pub(crate) async fn open(
    &self,
    id: JoystickId,
  ) -> Result<(SdlOpenedGamepadHandle, SdlRumbleCapabilities), SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Open { id, reply })
      .await?
  }

  #[cfg(test)]
  async fn shutdown(&self) -> Result<(), SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Shutdown { reply })
      .await
  }

  pub(crate) async fn set_rumble_state(
    &self,
    id: JoystickId,
    generation: u64,
    state: SdlRumbleState,
    duration: u32,
  ) -> Result<(), SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::SetRumbleState {
        id,
        generation,
        state,
        duration,
        reply,
      })
      .await?
  }

  pub(crate) async fn close(&self, id: JoystickId, generation: u64) -> Result<(), SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Close {
        id,
        generation,
        reply,
      })
      .await?;
    Ok(())
  }

  /// Fire-and-forget close usable from synchronous contexts (e.g. `Drop`).
  /// Closing an already-closed, removed, or superseded (stale generation) id
  /// is a no-op on the thread side.
  pub(crate) fn close_now(&self, id: JoystickId, generation: u64) {
    // The reply channel is immediately dropped; the thread's reply send is
    // ignored (the receiver may already be gone).
    let (reply_tx, _) = oneshot::channel();
    if self
      .cmd_tx
      .send(SdlCommand::Close {
        id,
        generation,
        reply: reply_tx,
      })
      .is_err()
    {
      warn!(
        "SDL gamepad thread already stopped; cannot close gamepad {}",
        id.0
      );
    }
  }
}

struct OpenPadState {
  pad: Box<dyn DriverGamepad>,
  generation: u64,
  removed_tx: watch::Sender<bool>,
  last_main: (u16, u16),
  main_set_at: u64,
  last_triggers: (u16, u16),
  triggers_set_at: u64,
}

/// Pure rumble-refresh decision for one independent main or trigger pair:
/// given the last accepted command, when it was armed, and the current time,
/// decide whether that pair must be re-armed.
///
/// Zero-speed commands never refresh (the pair is stopped; letting the effect
/// lapse is exactly what we want). Non-zero commands re-arm after
/// [`RUMBLE_KEEPALIVE_INTERVAL_MS`], safely before the finite arm duration lapses.
fn refresh_decision(last_rumble: (u16, u16), last_set_at: u64, now_ms: u64) -> Option<(u16, u16)> {
  if last_rumble == (0, 0) {
    return None;
  }
  if now_ms.saturating_sub(last_set_at) >= RUMBLE_KEEPALIVE_INTERVAL_MS {
    Some(last_rumble)
  } else {
    None
  }
}

fn mark_removed(state: OpenPadState) {
  // Receiver may already be gone; that's fine.
  let _ = state.removed_tx.send(true);
  // Dropping the state drops the DriverGamepad, closing the OS handle.
}

/// Best-effort stop of an actively rumbling gamepad before its pad is
/// dropped. Rumble is armed with a finite duration, so hardware quiets even
/// if this fails, but an explicit stop avoids up to a full arm period of
/// vibration after a disconnect while rumbling.
fn stop_and_drop(mut state: OpenPadState) {
  if state.pad.has_rumble() {
    let _ = state.pad.rumble(0, 0, RUMBLE_DURATION_MS);
  }
  if state.pad.has_rumble_triggers() {
    let _ = state.pad.rumble_triggers(0, 0, RUMBLE_DURATION_MS);
  }
  mark_removed(state);
}

fn teardown(open_pads: &mut HashMap<JoystickId, OpenPadState>) {
  for (_, state) in open_pads.drain() {
    stop_and_drop(state);
  }
}

/// The SDL thread's command loop.
fn sdl_thread_loop(
  task_tx: SdlTaskHandle,
  mut driver: Box<dyn SdlDriver>,
  clock: Box<dyn SdlClock>,
  cmd_rx: mpsc::Receiver<SdlCommand>,
) {
  let mut open_pads: HashMap<JoystickId, OpenPadState> = HashMap::new();
  let mut last_poll_ms: u64 = 0;
  // Monotonic per-open lease counter: lets the thread reject commands from
  // handles belonging to a superseded open of the same id.
  let mut next_generation: u64 = 0;
  loop {
    let now = clock.now_ms();

    // Periodic work runs on every wake (command or timeout), so tests can
    // drive it deterministically by advancing the injected clock and sending
    // a probe command.
    if now.saturating_sub(last_poll_ms) >= CONNECTED_POLL_INTERVAL_MS {
      last_poll_ms = now;
      let mut removed = Vec::new();
      for (id, state) in open_pads.iter_mut() {
        if !state.pad.connected() {
          info!("SDL gamepad {} has disconnected.", id.0);
          removed.push(*id);
        }
      }
      for id in removed {
        if let Some(state) = open_pads.remove(&id) {
          stop_and_drop(state);
        }
      }
    }

    // Refresh any non-zero rumble whose re-arm deadline has arrived. Errors
    // are treated as device loss: mark removed and drop the pad.
    let mut rumbles_to_refresh = Vec::new();
    for (id, state) in open_pads.iter() {
      if let Some(cmd) = refresh_decision(state.last_main, state.main_set_at, now) {
        rumbles_to_refresh.push((*id, false, cmd));
      }
      if let Some(cmd) = refresh_decision(state.last_triggers, state.triggers_set_at, now) {
        rumbles_to_refresh.push((*id, true, cmd));
      }
    }
    for (id, triggers, (low, high)) in rumbles_to_refresh {
      let Some(state) = open_pads.get_mut(&id) else {
        continue;
      };
      let result = if triggers {
        if !state.pad.has_rumble_triggers() {
          continue;
        }
        state.pad.rumble_triggers(low, high, RUMBLE_DURATION_MS)
      } else {
        if !state.pad.has_rumble() {
          continue;
        }
        state.pad.rumble(low, high, RUMBLE_DURATION_MS)
      };
      match result {
        Ok(()) => {
          if triggers {
            state.triggers_set_at = now;
          } else {
            state.main_set_at = now;
          }
        }
        Err(e) => {
          warn!("SDL gamepad {} rumble refresh failed: {}", id.0, e);
          if let Some(state) = open_pads.remove(&id) {
            stop_and_drop(state);
          }
        }
      }
    }

    // Wait for the next command (or wake timeout), then handle it.
    match cmd_rx.recv_timeout(Duration::from_millis(COMMAND_WAKE_MS)) {
      Ok(cmd) => match cmd {
        #[cfg(test)]
        SdlCommand::Shutdown { reply } => {
          teardown(&mut open_pads);
          let _ = reply.send(());
          break;
        }
        SdlCommand::Scan { reply } => {
          let result = driver.enumerate().map_err(|e| {
            warn!("SDL gamepad enumeration failed: {}", e);
            SdlTaskError::Scan(e)
          });
          let reply_value = result.map(|ids| {
            ids
              .into_iter()
              .filter_map(|id| {
                // macOS: wired pads enumerate via hidapi but cannot rumble -
                // Apple exposes only read-only shortened HID reports for them,
                // and working rumble requires GCController, whose discovery
                // only fires from a main-thread runloop this architecture
                // deliberately does not host. Skip wired pads so no dead
                // devices appear; Bluetooth pads work fully. Users with a
                // wired controller can pair the same pad via Bluetooth.
                let capabilities = if let Some(state) = open_pads.get(&id) {
                  SdlRumbleCapabilities {
                    rumble: state.pad.has_rumble(),
                    trigger_rumble: state.pad.has_rumble_triggers(),
                  }
                } else {
                  let pad = match driver.open(id) {
                    Ok(pad) => pad,
                    Err(e) => {
                      warn!("SDL gamepad {} probe open failed: {}", id.0, e);
                      return None;
                    }
                  };
                  let connection = pad.connection_state();
                  #[cfg(target_os = "macos")]
                  if connection == DriverConnection::Wired {
                    warn!(
                      "Skipping wired SDL gamepad {} on macOS: wired rumble is not possible without GCController (pair the controller via Bluetooth instead).",
                      id.0
                    );
                    return None;
                  }
                  #[cfg(not(target_os = "macos"))]
                  let _ = connection;
                  let capabilities = SdlRumbleCapabilities {
                    rumble: pad.has_rumble(),
                    trigger_rumble: pad.has_rumble_triggers(),
                  };
                  drop(pad);
                  if !capabilities.any() {
                    info!("SDL gamepad {} has no rumble capability, skipping", id.0);
                    return None;
                  }
                  capabilities
                };
                let name = match driver.name_for_id(id) {
                  Ok(name) if !name.trim().is_empty() => name,
                  Ok(_) => {
                    warn!("SDL gamepad {} name lookup returned an empty name", id.0);
                    format!("SDL Gamepad {}", id.0)
                  },
                  Err(e) => {
                    // A failed name lookup never drops the device: log and
                    // fall back to a deterministic name.
                    warn!("SDL gamepad {} name lookup failed: {}", id.0, e);
                    format!("SDL Gamepad {}", id.0)
                  }
                };
                Some(SdlGamepadDesc {
                  id,
                  name,
                  capabilities,
                  is_open: open_pads.contains_key(&id),
                })
              })
              .collect::<Vec<_>>()
          });
          let _ = reply.send(reply_value);
        }
        SdlCommand::Open { id, reply } => {
          if open_pads.contains_key(&id) {
            // Single lease per id: a duplicate open only happens after the
            // previous device fully disconnected and closed, and rejecting
            // keeps Close { id } unambiguous.
            let _ = reply.send(Err(SdlTaskError::AlreadyOpen(id)));
            continue;
          }
          match driver.open(id) {
            Ok(pad) => {
              let capabilities = SdlRumbleCapabilities {
                rumble: pad.has_rumble(),
                trigger_rumble: pad.has_rumble_triggers(),
              };
              if !capabilities.any() {
                drop(pad);
                let _ = reply.send(Err(SdlTaskError::NoRumbleCapability(id)));
                continue;
              }
              next_generation += 1;
              let generation = next_generation;
              let (removed_tx, removed_rx) = watch::channel(false);
              open_pads.insert(
                id,
                OpenPadState {
                  pad,
                  generation,
                  removed_tx,
                  last_main: (0, 0),
                  main_set_at: now,
                  last_triggers: (0, 0),
                  triggers_set_at: now,
                },
              );
              let handle = SdlOpenedGamepadHandle {
                id,
                generation,
                task: task_tx.clone(),
                removed_rx,
              };
              if reply.send(Ok((handle, capabilities))).is_err() {
                // The connect waiter is gone (future cancelled): nobody can
                // ever command or close this pad. Drop the lease now instead
                // of blocking future opens with AlreadyOpen until the device
                // physically disappears.
                if let Some(state) = open_pads.remove(&id) {
                  stop_and_drop(state);
                }
              }
            }
            Err(e) => {
              let _ = reply.send(Err(SdlTaskError::Open(e)));
            }
          }
        }
        SdlCommand::SetRumbleState {
          id,
          generation,
          state: desired,
          duration,
          reply,
        } => {
          let Some(state) = open_pads.get_mut(&id) else {
            let _ = reply.send(Err(SdlTaskError::Removed(id)));
            continue;
          };
          if state.generation != generation {
            // Stale handle from a superseded open of the same id.
            let _ = reply.send(Err(SdlTaskError::Removed(id)));
            continue;
          }
          let now = clock.now_ms();
          let mut errors = Vec::new();
          if state.pad.has_rumble() {
            match state.pad.rumble(desired.low, desired.high, duration) {
              Ok(()) => {
                state.last_main = (desired.low, desired.high);
                state.main_set_at = now;
              }
              Err(e) => errors.push(format!("main: {e}")),
            }
          }
          if state.pad.has_rumble_triggers() {
            match state
              .pad
              .rumble_triggers(desired.left_trigger, desired.right_trigger, duration)
            {
              Ok(()) => {
                state.last_triggers = (desired.left_trigger, desired.right_trigger);
                state.triggers_set_at = now;
              }
              Err(e) => errors.push(format!("triggers: {e}")),
            }
          }
          if errors.is_empty() {
            let _ = reply.send(Ok(()));
          } else {
            let _ = reply.send(Err(SdlTaskError::Rumble(errors.join("; "))));
            if let Some(state) = open_pads.remove(&id) {
              stop_and_drop(state);
            }
          }
        }
        SdlCommand::Close {
          id,
          generation,
          reply,
        } => {
          // Idempotent: closing an already-closed, removed, or superseded id
          // is a no-op that still replies Ok.
          if let Some(state) = open_pads.remove(&id) {
            if state.generation == generation {
              stop_and_drop(state);
            } else {
              // Stale close: reinstate the newer lease untouched.
              open_pads.insert(id, state);
            }
          }
          let _ = reply.send(());
        }
      },
      Err(mpsc::RecvTimeoutError::Timeout) => {
        // Plain wake; periodic work will be re-checked at the top of the loop.
      }
      Err(mpsc::RecvTimeoutError::Disconnected) => {
        info!("SDL gamepad thread command channel closed; exiting.");
        teardown(&mut open_pads);
        break;
      }
    }
  }
}

/// Spawn the SDL thread, running `factory` on it to build the driver.
///
/// Only the `Send` factory closure moves into the new thread; every SDL value
/// it produces stays there for its whole lifetime. The returned handle is
/// non-global (tests spawn their own instances with fake drivers).
pub(crate) fn spawn_sdl_task<F>(
  factory: F,
  clock: Box<dyn SdlClock>,
) -> Result<SdlTaskHandle, SdlTaskInitError>
where
  F: FnOnce() -> Result<Box<dyn SdlDriver>, SdlTaskInitError> + Send + 'static,
{
  spawn_sdl_task_inner(factory, clock).map(|(handle, _join)| handle)
}

#[cfg(test)]
fn spawn_sdl_task_with_join<F>(
  factory: F,
  clock: Box<dyn SdlClock>,
) -> Result<(SdlTaskHandle, std::thread::JoinHandle<()>), SdlTaskInitError>
where
  F: FnOnce() -> Result<Box<dyn SdlDriver>, SdlTaskInitError> + Send + 'static,
{
  spawn_sdl_task_inner(factory, clock)
}

fn spawn_sdl_task_inner<F>(
  factory: F,
  clock: Box<dyn SdlClock>,
) -> Result<(SdlTaskHandle, std::thread::JoinHandle<()>), SdlTaskInitError>
where
  F: FnOnce() -> Result<Box<dyn SdlDriver>, SdlTaskInitError> + Send + 'static,
{
  let (cmd_tx, cmd_rx) = mpsc::channel::<SdlCommand>();
  let (init_tx, init_rx) = mpsc::channel::<Result<(), SdlTaskInitError>>();
  let loop_tx = SdlTaskHandle {
    cmd_tx: cmd_tx.clone(),
  };
  let join = std::thread::Builder::new()
    .name("buttplug-sdl-gamepad".to_string())
    .spawn(move || {
      let driver = match factory() {
        Ok(driver) => {
          if init_tx.send(Ok(())).is_err() {
            // Caller went away; still run so the thread doesn't dangle.
          }
          driver
        }
        Err(e) => {
          let _ = init_tx.send(Err(e));
          return;
        }
      };
      sdl_thread_loop(loop_tx, driver, clock, cmd_rx);
    })
    .map_err(|e| SdlTaskInitError(format!("failed to spawn SDL thread: {e}")))?;
  // Startup handshake: blocks only for the duration of SDL initialization.
  init_rx
    .recv()
    .map_err(|_| SdlTaskInitError("SDL thread exited before initialization".to_owned()))?
    .map_err(|e| e)?;
  Ok((SdlTaskHandle { cmd_tx }, join))
}

// ---------------------------------------------------------------------------
// Production driver: real SDL3 calls, confined to the SDL thread.
// ---------------------------------------------------------------------------

struct Sdl3Driver {
  // Held to keep SDL alive; dropping the last reference would SDL_Quit, which
  // only happens at thread exit.
  _sdl: sdl3::Sdl,
  gamepads: sdl3::GamepadSubsystem,
}

impl SdlDriver for Sdl3Driver {
  fn enumerate(&mut self) -> Result<Vec<JoystickId>, String> {
    self.gamepads.gamepads().map_err(|e| e.to_string())
  }

  fn name_for_id(&mut self, id: JoystickId) -> Result<String, String> {
    self.gamepads.name_for_id(id).map_err(|e| e.to_string())
  }

  fn open(&mut self, id: JoystickId) -> Result<Box<dyn DriverGamepad>, String> {
    self
      .gamepads
      .open(id)
      .map(|pad| Box::new(Sdl3Gamepad { pad }) as Box<dyn DriverGamepad>)
      .map_err(|e| e.to_string())
  }
}

struct Sdl3Gamepad {
  pad: sdl3::gamepad::Gamepad,
}

impl DriverGamepad for Sdl3Gamepad {
  fn has_rumble(&self) -> bool {
    // SAFETY: Pure property-table read of this opened gamepad, exclusively
    // owned by this SDL thread, so no concurrent SDL access is possible.
    // Missing properties resolve to false; this does not activate motors.
    unsafe { self.pad.has_rumble() }
  }

  fn has_rumble_triggers(&self) -> bool {
    // SAFETY: Pure property-table read of this opened gamepad, exclusively
    // owned by this SDL thread, so no concurrent SDL access is possible.
    // Missing properties resolve to false; this does not activate motors.
    unsafe { self.pad.has_rumble_triggers() }
  }

  fn rumble_triggers(&mut self, left: u16, right: u16, duration_ms: u32) -> Result<(), String> {
    self
      .pad
      .set_rumble_triggers(left, right, duration_ms)
      .map_err(|e| e.to_string())
  }

  fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String> {
    self
      .pad
      .set_rumble(low, high, duration_ms)
      .map_err(|e| e.to_string())
  }

  fn connected(&self) -> bool {
    self.pad.connected()
  }

  fn connection_state(&self) -> DriverConnection {
    match self.pad.connection_state() {
      Ok(sdl3::joystick::ConnectionState::Wired) => DriverConnection::Wired,
      Ok(sdl3::joystick::ConnectionState::Wireless) => DriverConnection::Wireless,
      _ => DriverConnection::Unknown,
    }
  }
}

/// Production factory: sets the background-events hint (SDL guidance is to do
/// this before initialization so hotplug works while unfocused/headless),
/// initializes SDL + the gamepad subsystem, and builds the driver.
///
/// On macOS, SDL3 routes wired gamepads to GCController (MFI) by default, and
/// hidapi device drivers decline them while MFI is enabled (see the
/// `SDL_PLATFORM_MACOS && SDL_JOYSTICK_MFI` guard in SDL's hidapi drivers:
/// wired pads enumerate with DevSrvsID paths). GCController discovery is
/// delivered through Cocoa runloop notifications, which this headless,
/// no-video process never spins - so with the default policy no gamepads are
/// ever discovered here. Disabling MFI routes gamepads to hidapi, which
/// enumerates synchronously and works headless (verified on hardware: a wired
/// Xbox One S enumerates and `set_rumble` succeeds with this hint). iOS keeps
/// the MFI default, where GCController is the only gamepad backend.
fn production_sdl_factory() -> Result<Box<dyn SdlDriver>, SdlTaskInitError> {
  // SDL installs SIGINT/SIGTERM handlers by default and turns those signals
  // into SDL quit events. This backend is headless and intentionally never
  // pumps SDL events, so leave signal ownership with the host application
  // (intiface-engine uses Tokio's ctrl_c handler).
  sdl3::hint::set(sdl3::hint::names::NO_SIGNAL_HANDLERS, "1");
  sdl3::hint::set(sdl3::hint::names::JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
  #[cfg(target_os = "macos")]
  sdl3::hint::set(sdl3::hint::names::JOYSTICK_MFI, "0");
  let sdl = sdl3::init().map_err(|e| SdlTaskInitError(e.to_string()))?;
  let gamepads = sdl.gamepad().map_err(|e| SdlTaskInitError(e.to_string()))?;
  Ok(Box::new(Sdl3Driver {
    _sdl: sdl,
    gamepads,
  }))
}

// ---------------------------------------------------------------------------
// Process-global publication.
// ---------------------------------------------------------------------------

type PublishedSdlTask = Result<Arc<SdlTaskHandle>, Arc<SdlTaskInitError>>;

static GLOBAL_SDL_TASK: OnceLock<PublishedSdlTask> = OnceLock::new();

/// Publication decision: run the factory once, publish a usable handle on
/// success, or a permanent, logged inert state on failure. Retrying
/// `SDL_Init` after a failure mid-process is not attempted.
///
/// Generic over the cell so tests can exercise the decision on a local
/// `OnceLock` without mutating the process-global one.
fn publish_sdl_task<F>(cell: &OnceLock<PublishedSdlTask>, factory: F) -> &PublishedSdlTask
where
  F: FnOnce() -> Result<SdlTaskHandle, SdlTaskInitError>,
{
  cell.get_or_init(|| match factory() {
    Ok(handle) => {
      info!("SDL gamepad manager initialized.");
      Ok(Arc::new(handle))
    }
    Err(e) => {
      error!("SDL gamepad manager failed to initialize and is disabled: {e}");
      Err(Arc::new(e))
    }
  })
}

/// The process-lifetime SDL task. First use spawns the thread; the handle is
/// never dropped, so the thread (and SDL context) lives until process exit.
pub(crate) fn global_sdl_task() -> &'static PublishedSdlTask {
  publish_sdl_task(&GLOBAL_SDL_TASK, || {
    spawn_sdl_task(
      production_sdl_factory,
      Box::new(SystemClock {
        start: std::time::Instant::now(),
      }),
    )
  })
}

// ---------------------------------------------------------------------------
// Outer seam: async backend over the task handle.
// ---------------------------------------------------------------------------

use async_trait::async_trait;

/// An opened gamepad as seen by the hardware layer: mockable, with no SDL
/// dependency. Production wraps [`SdlOpenedGamepadHandle`].
#[async_trait]
pub(crate) trait SdlOpenedGamepad: Send + Sync + std::fmt::Debug {
  async fn set_rumble_state(
    &self,
    state: SdlRumbleState,
    duration_ms: u32,
  ) -> Result<(), SdlTaskError>;
  async fn close(&self) -> Result<(), SdlTaskError>;
  /// Fire-and-forget close usable from synchronous contexts (e.g. `Drop`).
  fn close_now(&self);
  /// Receiver that yields `true` when the gamepad is closed or disconnected.
  fn removed(&self) -> watch::Receiver<bool>;
}

/// Async gamepad surface used by the communication manager and hardware.
///
/// Production wraps [`SdlTaskHandle`]; tests provide mock implementations so
/// all buttplug-side behavior can be tested without SDL or hardware. The SDL
/// thread's internal invariants are tested separately through the
/// [`SdlDriver`] seam against the real command loop.
#[async_trait]
pub(crate) trait SdlGamepadBackend: Send + Sync {
  /// Whether the underlying SDL task initialized successfully.
  fn initialized(&self) -> bool;
  async fn gamepads(&self) -> Result<Vec<SdlGamepadDesc>, SdlTaskError>;
  async fn open(
    &self,
    id: JoystickId,
  ) -> Result<(Arc<dyn SdlOpenedGamepad>, SdlRumbleCapabilities), SdlTaskError>;
}

/// Production opened-gamepad wrapper over the task handle.
#[derive(Debug)]
struct TaskOpenedGamepad {
  handle: SdlOpenedGamepadHandle,
}

#[async_trait]
impl SdlOpenedGamepad for TaskOpenedGamepad {
  async fn set_rumble_state(
    &self,
    state: SdlRumbleState,
    duration_ms: u32,
  ) -> Result<(), SdlTaskError> {
    self.handle.set_rumble_state(state, duration_ms).await
  }

  async fn close(&self) -> Result<(), SdlTaskError> {
    self.handle.close().await
  }

  fn close_now(&self) {
    self.handle.close_now();
  }

  fn removed(&self) -> watch::Receiver<bool> {
    self.handle.removed()
  }
}

/// Production backend over the process-global SDL task.
pub(crate) struct SdlTaskBackend {
  publication: &'static PublishedSdlTask,
}

impl SdlTaskBackend {
  pub(crate) fn global() -> Self {
    Self {
      publication: global_sdl_task(),
    }
  }
}

#[async_trait]
impl SdlGamepadBackend for SdlTaskBackend {
  fn initialized(&self) -> bool {
    self.publication.is_ok()
  }

  async fn gamepads(&self) -> Result<Vec<SdlGamepadDesc>, SdlTaskError> {
    match self.publication {
      Ok(handle) => handle.scan().await,
      Err(e) => Err(SdlTaskError::Init(e.to_string())),
    }
  }

  async fn open(
    &self,
    id: JoystickId,
  ) -> Result<(Arc<dyn SdlOpenedGamepad>, SdlRumbleCapabilities), SdlTaskError> {
    match self.publication {
      Ok(handle) => {
        let (handle, capabilities) = handle.open(id).await?;
        Ok((Arc::new(TaskOpenedGamepad { handle }), capabilities))
      }
      Err(e) => Err(SdlTaskError::Init(e.to_string())),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
  };

  // -------------------------------------------------------------------
  // Fakes
  // -------------------------------------------------------------------

  #[derive(Default)]
  struct FakeDriverState {
    enumerate_ids: Vec<JoystickId>,
    enumerate_fail: bool,
    name_fail_ids: Vec<JoystickId>,
    open_fail_ids: Vec<JoystickId>,
    connected: HashMap<JoystickId, bool>,
    // Log of (id, low, high, duration) rumble calls.
    rumble_log: Vec<(JoystickId, u16, u16, u32)>,
    rumble_fail: bool,
    trigger_rumble_fail: bool,
    rumble_caps: HashMap<JoystickId, SdlRumbleCapabilities>,
    name_override: HashMap<JoystickId, String>,
    rumble_attempts: Vec<(JoystickId, u16, u16, u32)>,
    trigger_rumble_attempts: Vec<(JoystickId, u16, u16, u32)>,
    trigger_rumble_log: Vec<(JoystickId, u16, u16, u32)>,
    wired_ids: Vec<JoystickId>,
  }

  struct FakeDriver(Arc<Mutex<FakeDriverState>>);

  struct FakeGamepad {
    id: JoystickId,
    state: Arc<Mutex<FakeDriverState>>,
  }

  impl DriverGamepad for FakeGamepad {
    fn has_rumble(&self) -> bool {
      self
        .state
        .lock()
        .unwrap()
        .rumble_caps
        .get(&self.id)
        .map(|caps| caps.rumble)
        .unwrap_or(true)
    }

    fn has_rumble_triggers(&self) -> bool {
      self
        .state
        .lock()
        .unwrap()
        .rumble_caps
        .get(&self.id)
        .map(|caps| caps.trigger_rumble)
        .unwrap_or(false)
    }

    fn rumble_triggers(&mut self, left: u16, right: u16, duration_ms: u32) -> Result<(), String> {
      let mut state = self.state.lock().unwrap();
      state
        .trigger_rumble_attempts
        .push((self.id, left, right, duration_ms));
      if state.trigger_rumble_fail {
        return Err("trigger rumble failed".to_owned());
      }
      state
        .trigger_rumble_log
        .push((self.id, left, right, duration_ms));
      Ok(())
    }

    fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String> {
      let mut state = self.state.lock().unwrap();
      state
        .rumble_attempts
        .push((self.id, low, high, duration_ms));
      if state.rumble_fail {
        return Err("rumble failed".to_owned());
      }
      state.rumble_log.push((self.id, low, high, duration_ms));
      Ok(())
    }

    fn connection_state(&self) -> DriverConnection {
      let state = self.state.lock().unwrap();
      if state.wired_ids.contains(&self.id) {
        DriverConnection::Wired
      } else {
        DriverConnection::Wireless
      }
    }

    fn connected(&self) -> bool {
      *self
        .state
        .lock()
        .unwrap()
        .connected
        .get(&self.id)
        .unwrap_or(&true)
    }
  }

  impl SdlDriver for FakeDriver {
    fn enumerate(&mut self) -> Result<Vec<JoystickId>, String> {
      let state = self.0.lock().unwrap();
      if state.enumerate_fail {
        Err("enumeration failed".to_owned())
      } else {
        Ok(state.enumerate_ids.clone())
      }
    }

    fn name_for_id(&mut self, id: JoystickId) -> Result<String, String> {
      let state = self.0.lock().unwrap();
      if state.name_fail_ids.contains(&id) {
        Err("name lookup failed".to_owned())
      } else {
        Ok(
          state
            .name_override
            .get(&id)
            .cloned()
            .unwrap_or_else(|| format!("SDL Fake Pad {}", id.0)),
        )
      }
    }

    fn open(&mut self, id: JoystickId) -> Result<Box<dyn DriverGamepad>, String> {
      let state = self.0.lock().unwrap();
      if state.open_fail_ids.contains(&id) {
        Err("open failed".to_owned())
      } else {
        Ok(Box::new(FakeGamepad {
          id,
          state: self.0.clone(),
        }))
      }
    }
  }

  /// Injected clock: an atomic millisecond counter the test advances.
  #[derive(Clone, Default)]
  struct FakeClock(Arc<AtomicU64>);

  impl SdlClock for FakeClock {
    fn now_ms(&self) -> u64 {
      self.0.load(Ordering::SeqCst)
    }
  }

  impl FakeClock {
    fn advance_to(&self, ms: u64) {
      self.0.store(ms, Ordering::SeqCst);
    }
  }

  fn spawn_fake(state: Arc<Mutex<FakeDriverState>>, clock: FakeClock) -> SdlTaskHandle {
    spawn_sdl_task(
      move || {
        let state = state;
        Ok(Box::new(FakeDriver(state)) as Box<dyn SdlDriver>)
      },
      Box::new(clock),
    )
    .expect("fake driver factory always succeeds")
  }

  fn id(n: u32) -> JoystickId {
    joystick_id(n)
  }

  fn caps(rumble: bool, trigger_rumble: bool) -> SdlRumbleCapabilities {
    SdlRumbleCapabilities {
      rumble,
      trigger_rumble,
    }
  }

  async fn barrier(handle: &SdlTaskHandle) {
    // Two commands guarantee a loop-top periodic pass after the clock change.
    handle.scan().await.unwrap();
    handle.scan().await.unwrap();
  }

  #[tokio::test]
  async fn sdl_scan_capability_matrix() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(1), id(2), id(3), id(4)],
      rumble_caps: HashMap::from([
        (id(1), caps(true, false)),
        (id(2), caps(false, true)),
        (id(3), caps(true, true)),
        (id(4), caps(false, false)),
      ]),
      ..Default::default()
    }));
    let handle = spawn_fake(state.clone(), FakeClock::default());
    let found = handle.scan().await.unwrap();
    assert_eq!(
      found
        .iter()
        .map(|pad| (pad.id, pad.capabilities))
        .collect::<Vec<_>>(),
      vec![
        (id(1), caps(true, false)),
        (id(2), caps(false, true)),
        (id(3), caps(true, true))
      ]
    );
    assert!(state.lock().unwrap().rumble_attempts.is_empty());
    assert!(state.lock().unwrap().trigger_rumble_attempts.is_empty());
    handle.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn sdl_probe_failure_retries() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(1)],
      open_fail_ids: vec![id(1)],
      ..Default::default()
    }));
    let handle = spawn_fake(state.clone(), FakeClock::default());
    assert!(handle.scan().await.unwrap().is_empty());
    state.lock().unwrap().open_fail_ids.clear();
    assert_eq!(handle.scan().await.unwrap().len(), 1);
    handle.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn sdl_connect_rechecks_capabilities() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(1)],
      rumble_caps: HashMap::from([(id(1), caps(true, true))]),
      ..Default::default()
    }));
    let handle = spawn_fake(state.clone(), FakeClock::default());
    assert_eq!(
      handle.scan().await.unwrap()[0].capabilities,
      caps(true, true)
    );
    state
      .lock()
      .unwrap()
      .rumble_caps
      .insert(id(1), caps(true, false));
    let (opened, actual) = handle.open(id(1)).await.unwrap();
    assert_eq!(actual, caps(true, false));
    opened.close().await.unwrap();
    state
      .lock()
      .unwrap()
      .rumble_caps
      .insert(id(1), caps(false, false));
    assert!(matches!(
      handle.open(id(1)).await,
      Err(SdlTaskError::NoRumbleCapability(_))
    ));
    state
      .lock()
      .unwrap()
      .rumble_caps
      .insert(id(1), caps(false, true));
    assert_eq!(handle.open(id(1)).await.unwrap().1, caps(false, true));
    handle.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn sdl_name_fallback_matrix() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(1), id(2), id(3)],
      name_fail_ids: vec![id(2)],
      name_override: HashMap::from([
        (id(1), " Valid Pad ".to_owned()),
        (id(3), " \t ".to_owned()),
      ]),
      ..Default::default()
    }));
    let handle = spawn_fake(state, FakeClock::default());
    assert_eq!(
      handle
        .scan()
        .await
        .unwrap()
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>(),
      vec![" Valid Pad ", "SDL Gamepad 2", "SDL Gamepad 3"]
    );
    handle.shutdown().await.unwrap();
  }

  #[tokio::test]
  async fn sdl_lifecycle_pair_matrix() {
    for capability in [caps(true, false), caps(false, true), caps(true, true)] {
      let state = Arc::new(Mutex::new(FakeDriverState {
        rumble_caps: HashMap::from([(id(1), capability)]),
        ..Default::default()
      }));
      let handle = spawn_fake(state.clone(), FakeClock::default());
      let (opened, _) = handle.open(id(1)).await.unwrap();
      opened
        .set_rumble_state(
          SdlRumbleState {
            low: 500,
            right_trigger: 700,
            ..Default::default()
          },
          RUMBLE_DURATION_MS,
        )
        .await
        .unwrap();
      opened
        .set_rumble_state(SdlRumbleState::default(), RUMBLE_DURATION_MS)
        .await
        .unwrap();
      opened.close().await.unwrap();
      {
        let state = state.lock().unwrap();
        assert_eq!(
          state.rumble_attempts.len(),
          if capability.rumble { 3 } else { 0 }
        );
        assert_eq!(
          state.trigger_rumble_attempts.len(),
          if capability.trigger_rumble { 3 } else { 0 }
        );
        if capability.rumble {
          assert_eq!(
            state.rumble_attempts.last(),
            Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
          );
        }
        if capability.trigger_rumble {
          assert_eq!(
            state.trigger_rumble_attempts.last(),
            Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
          );
        }
      }
      handle.shutdown().await.unwrap();
    }
  }

  #[tokio::test]
  async fn sdl_keepalive_pair_matrix() {
    for capability in [caps(true, false), caps(false, true), caps(true, true)] {
      for active_triggers in [false, true] {
        let state = Arc::new(Mutex::new(FakeDriverState {
          rumble_caps: HashMap::from([(id(1), capability)]),
          ..Default::default()
        }));
        let clock = FakeClock::default();
        let handle = spawn_fake(state.clone(), clock.clone());
        let (opened, _) = handle.open(id(1)).await.unwrap();
        let desired = SdlRumbleState {
          low: if active_triggers { 0 } else { 500 },
          right_trigger: if active_triggers { 700 } else { 0 },
          ..Default::default()
        };
        opened
          .set_rumble_state(desired, RUMBLE_DURATION_MS)
          .await
          .unwrap();
        clock.advance_to(RUMBLE_KEEPALIVE_INTERVAL_MS + 1);
        barrier(&handle).await;
        barrier(&handle).await;
        {
          let state = state.lock().unwrap();
          assert_eq!(
            state.rumble_attempts.len(),
            if capability.rumble {
              if active_triggers { 1 } else { 2 }
            } else {
              0
            }
          );
          assert_eq!(
            state.trigger_rumble_attempts.len(),
            if capability.trigger_rumble {
              if active_triggers { 2 } else { 1 }
            } else {
              0
            }
          );
        }
        handle.shutdown().await.unwrap();
      }
    }
  }

  #[tokio::test]
  async fn sdl_pair_failure_cleanup() {
    for main_failure in [false, true] {
      let state = Arc::new(Mutex::new(FakeDriverState {
        rumble_caps: HashMap::from([(id(1), caps(true, true))]),
        ..Default::default()
      }));
      let handle = spawn_fake(state.clone(), FakeClock::default());
      let (opened, _) = handle.open(id(1)).await.unwrap();
      let removed = opened.removed();
      let desired = SdlRumbleState {
        low: 500,
        right_trigger: 700,
        ..Default::default()
      };
      opened
        .set_rumble_state(desired, RUMBLE_DURATION_MS)
        .await
        .unwrap();
      {
        let mut state = state.lock().unwrap();
        state.rumble_fail = main_failure;
        state.trigger_rumble_fail = !main_failure;
      }
      assert!(matches!(
        opened.set_rumble_state(desired, RUMBLE_DURATION_MS).await,
        Err(SdlTaskError::Rumble(_))
      ));
      barrier(&handle).await;
      assert!(*removed.borrow());
      assert!(matches!(
        opened.set_rumble_state(desired, RUMBLE_DURATION_MS).await,
        Err(SdlTaskError::Removed(_))
      ));
      {
        let state = state.lock().unwrap();
        assert_eq!(
          state.rumble_attempts.last(),
          Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
        );
        assert_eq!(
          state.trigger_rumble_attempts.last(),
          Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
        );
        assert_eq!(state.rumble_attempts.len(), 3);
        assert_eq!(state.trigger_rumble_attempts.len(), 3);
      }
      handle.shutdown().await.unwrap();
    }
  }

  async fn shutdown_case(main_failure: bool) {
    let state = Arc::new(Mutex::new(FakeDriverState {
      rumble_caps: HashMap::from([(id(1), caps(true, true))]),
      ..Default::default()
    }));
    let driver_state = state.clone();
    let (handle, join) = spawn_sdl_task_with_join(
      move || Ok(Box::new(FakeDriver(driver_state))),
      Box::new(FakeClock::default()),
    )
    .unwrap();
    let (opened, _) = handle.open(id(1)).await.unwrap();
    let removed = opened.removed();
    opened
      .set_rumble_state(
        SdlRumbleState {
          low: 500,
          right_trigger: 700,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .unwrap();
    state.lock().unwrap().rumble_fail = main_failure;
    tokio::time::timeout(Duration::from_secs(5), handle.shutdown())
      .await
      .unwrap()
      .unwrap();
    // Bound the join without an uncancellable blocking task: poll
    // `is_finished` on the async timer and only call `join` once the thread
    // has actually exited, so a hung thread fails the test instead of
    // wedging the test runtime.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
      if join.is_finished() {
        join.join().expect("SDL thread should not panic");
        break;
      }
      assert!(
        tokio::time::Instant::now() < deadline,
        "SDL thread did not exit after shutdown"
      );
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(*removed.borrow());
    let state = state.lock().unwrap();
    assert_eq!(
      state.rumble_attempts.last(),
      Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
    );
    assert_eq!(
      state.trigger_rumble_attempts.last(),
      Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
    );
    assert_eq!(
      state.trigger_rumble_log.last(),
      Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
    );
    if !main_failure {
      assert_eq!(
        state.rumble_log.last(),
        Some(&(id(1), 0, 0, RUMBLE_DURATION_MS))
      );
    }
  }

  #[tokio::test]
  async fn sdl_task_shutdown_teardown_case() {
    for main_failure in [false, true] {
      shutdown_case(main_failure).await;
    }
  }

  // -------------------------------------------------------------------
  // Scan / name policy
  // -------------------------------------------------------------------

  #[tokio::test]
  async fn sdl_task_scan_replies_enumeration_error_and_recovers() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(1)],
      enumerate_fail: true,
      ..Default::default()
    }));
    let clock = FakeClock::default();
    let handle = spawn_fake(state.clone(), clock);

    // Failing enumeration surfaces as an Err reply.
    let err = handle.scan().await.expect_err("scan should fail");
    assert!(matches!(err, SdlTaskError::Scan(_)), "got {err:?}");

    // The same task recovers once the driver is healthy again.
    state.lock().unwrap().enumerate_fail = false;
    let descs = handle.scan().await.expect("scan should recover");
    assert_eq!(descs.len(), 1);
    assert_eq!(descs[0].id, id(1));
    assert_eq!(descs[0].name, "SDL Fake Pad 1");
  }

  // macOS-only behavior: wired pads are skipped at scan time because their
  // rumble cannot work under this architecture (see the scan handler).
  #[cfg(target_os = "macos")]
  #[tokio::test]
  async fn sdl_task_macos_scan_skips_wired_pads() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(20), id(21), id(22)],
      wired_ids: vec![id(21)],
      ..Default::default()
    }));
    let handle = spawn_fake(state, FakeClock::default());

    let descs = handle.scan().await.expect("scan should succeed");
    // 21 is wired and must be skipped; the wireless pads (and an
    // already-leased pad, not applicable here) come through.
    assert_eq!(
      descs.iter().map(|d| d.id).collect::<Vec<_>>(),
      vec![id(20), id(22)]
    );
  }

  // -------------------------------------------------------------------
  // Open / close / rumble lifecycle
  // -------------------------------------------------------------------

  #[tokio::test]
  async fn sdl_task_rejects_duplicate_open() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let handle = spawn_fake(state, FakeClock::default());

    handle.open(id(3)).await.expect("first open should succeed");
    let err = handle
      .open(id(3))
      .await
      .expect_err("duplicate open should fail");
    assert!(
      matches!(err, SdlTaskError::AlreadyOpen(found) if found == id(3)),
      "got {err:?}"
    );
  }

  #[tokio::test]
  async fn sdl_task_close_is_idempotent() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let handle = spawn_fake(state, FakeClock::default());

    let (opened, _) = handle.open(id(4)).await.expect("open should succeed");
    let removed = opened.removed();
    opened.close().await.expect("close should succeed");
    assert!(*removed.borrow());

    // Closing the same id again is Ok.
    handle
      .close(id(4), 1)
      .await
      .expect("second close should be ok");
    // Closing a never-opened id is Ok too.
    handle
      .close(id(99), 1)
      .await
      .expect("unknown close should be ok");
  }

  #[tokio::test]
  async fn sdl_task_rumble_after_removal_errors() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let handle = spawn_fake(state, FakeClock::default());

    handle.open(id(5)).await.expect("open should succeed");
    handle.close(id(5), 1).await.expect("close should succeed");

    let err = handle
      .set_rumble_state(
        id(5),
        0,
        SdlRumbleState {
          low: 100,
          high: 100,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect_err("rumble after close should fail");
    assert!(
      matches!(err, SdlTaskError::Removed(found) if found == id(5)),
      "got {err:?}"
    );
  }

  #[tokio::test]
  async fn sdl_task_close_stops_active_rumble() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let clock = FakeClock::default();
    let handle = spawn_fake(state.clone(), clock.clone());

    // Explicit close while rumbling emits a zero-speed stop before the pad
    // is dropped, so hardware does not vibrate out the remaining arm period.
    let (opened, _) = handle.open(id(13)).await.expect("open should succeed");
    opened
      .set_rumble_state(
        SdlRumbleState {
          low: 100,
          high: 100,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect("rumble should succeed");
    opened.close().await.expect("close should succeed");
    assert_eq!(
      state.lock().unwrap().rumble_log,
      vec![
        (id(13), 100, 100, RUMBLE_DURATION_MS),
        (id(13), 0, 0, RUMBLE_DURATION_MS),
      ]
    );

    // Connected-state removal while rumbling stops too.
    let (opened, _) = handle.open(id(14)).await.expect("open should succeed");
    opened
      .set_rumble_state(
        SdlRumbleState {
          low: 100,
          high: 100,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect("rumble should succeed");
    state.lock().unwrap().connected.insert(id(14), false);
    clock.advance_to(CONNECTED_POLL_INTERVAL_MS * 10);
    barrier(&handle).await;
    assert_eq!(
      state.lock().unwrap().rumble_log.last(),
      Some(&(id(14), 0, 0, RUMBLE_DURATION_MS)),
      "removal must stop active rumble"
    );
  }

  #[tokio::test]
  async fn sdl_task_cancelled_open_does_not_leak_lease() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let handle = spawn_fake(state, FakeClock::default());

    // Simulate a connect future cancelled mid-flight: the reply receiver is
    // dropped before the thread answers the Open.
    let (reply_tx, reply_rx) = oneshot::channel();
    drop(reply_rx);
    handle
      .cmd_tx
      .send(SdlCommand::Open {
        id: id(9),
        reply: reply_tx,
      })
      .expect("send open command");
    // Probe until the open has been processed.
    handle.scan().await.expect("probe scan should succeed");

    // The abandoned lease must have been cleaned up, so a real open succeeds
    // instead of being rejected as AlreadyOpen forever.
    handle
      .open(id(9))
      .await
      .expect("open after cancelled open must succeed");
  }

  #[tokio::test]
  async fn sdl_stale_generation_pair_isolation() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      rumble_caps: HashMap::from([(id(15), caps(true, true))]),
      ..Default::default()
    }));
    let handle = spawn_fake(state.clone(), FakeClock::default());

    // First lease: open, rumble, close (device stays connected).
    let (stale, _) = handle.open(id(15)).await.expect("open should succeed");
    stale
      .set_rumble_state(
        SdlRumbleState {
          low: 100,
          high: 100,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect("rumble should succeed");
    stale.close().await.expect("close should succeed");
    let log_len_after_first_lease = state.lock().unwrap().rumble_log.len();

    // Second lease for the same still-connected id.
    let (fresh, _) = handle.open(id(15)).await.expect("reopen should succeed");

    // Stale-handle rumble is rejected...
    let err = stale
      .set_rumble_state(
        SdlRumbleState {
          low: 1,
          high: 1,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect_err("stale rumble must fail");
    assert!(matches!(err, SdlTaskError::Removed(_)), "got {err:?}");
    // ...stale close is an Ok no-op that must NOT tear down the new lease...
    stale.close().await.expect("stale close is a no-op ok");
    assert_eq!(
      state.lock().unwrap().rumble_attempts.len(),
      log_len_after_first_lease
    );
    assert_eq!(
      state.lock().unwrap().trigger_rumble_attempts.len(),
      log_len_after_first_lease
    );
    // ...and the fresh lease still works.
    fresh
      .set_rumble_state(
        SdlRumbleState {
          low: 50,
          high: 50,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect("fresh lease rumble should succeed");

    let log = state.lock().unwrap().rumble_log.clone();
    assert_eq!(log.len(), log_len_after_first_lease + 1);
    assert_eq!(log.last(), Some(&(id(15), 50, 50, RUMBLE_DURATION_MS)));
    // Explicitly verify the fresh lease is still open.
    let err = handle
      .open(id(15))
      .await
      .expect_err("id still leased by fresh handle");
    assert!(matches!(err, SdlTaskError::AlreadyOpen(_)));
  }

  #[tokio::test]
  async fn sdl_task_connected_poll_marks_removed() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let clock = FakeClock::default();
    let handle = spawn_fake(state.clone(), clock.clone());

    let (opened, _) = handle.open(id(6)).await.expect("open should succeed");
    let mut removed = opened.removed();

    // Flip the device to disconnected, then advance the clock past the poll
    // interval and wake the loop with a scan probe. The poll runs on every
    // wake before commands are drained, so the removal must be observable by
    // the time the probe replies.
    state.lock().unwrap().connected.insert(id(6), false);
    clock.advance_to(CONNECTED_POLL_INTERVAL_MS + 1);
    handle.scan().await.expect("probe scan should succeed");

    loop {
      if *removed.borrow() {
        break;
      }
      // Poll interval wake-ups also happen on the plain timeout path; wait
      // for them without hanging forever on a bug.
      tokio::time::timeout(Duration::from_secs(5), removed.changed())
        .await
        .expect("removed signal must arrive within timeout")
        .expect("watch channel must stay live");
    }
    assert!(*removed.borrow());

    // After removal, rumble reports the typed Removed error, and close stays
    // idempotent.
    let err = handle
      .set_rumble_state(
        id(6),
        0,
        SdlRumbleState {
          low: 1,
          high: 1,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect_err("rumble after removal should fail");
    assert!(matches!(err, SdlTaskError::Removed(_)));
    handle
      .close(id(6), 1)
      .await
      .expect("close after removal is ok");
  }

  // -------------------------------------------------------------------
  // Rumble refresh
  // -------------------------------------------------------------------

  #[test]
  fn sdl_task_refresh_deadline_pure_function() {
    // Zero-speed commands never refresh.
    assert_eq!(refresh_decision((0, 0), 0, 1_000_000), None);
    // Before the deadline: no refresh.
    assert_eq!(
      refresh_decision((100, 200), 1_000, 1_000 + RUMBLE_KEEPALIVE_INTERVAL_MS - 1),
      None
    );
    // At the deadline: re-arm with the same speeds.
    assert_eq!(
      refresh_decision((100, 200), 1_000, 1_000 + RUMBLE_KEEPALIVE_INTERVAL_MS),
      Some((100, 200))
    );
    // Long past the deadline (e.g. after a stall): still re-arms.
    assert_eq!(
      refresh_decision((100, 200), 1_000, 1_000 + RUMBLE_DURATION_MS as u64 * 10),
      Some((100, 200))
    );
    // Clock never goes backwards: saturating subtraction, not panic.
    assert_eq!(refresh_decision((1, 1), 5_000, 1_000), None);
  }

  #[tokio::test]
  async fn sdl_task_refresh_rearms_before_expiry_at_loop_level() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let clock = FakeClock::default();
    let handle = spawn_fake(state.clone(), clock.clone());

    let (opened, _) = handle.open(id(7)).await.expect("open should succeed");
    opened
      .set_rumble_state(
        SdlRumbleState {
          low: 0x8000,
          high: 0x7fff,
          ..Default::default()
        },
        RUMBLE_DURATION_MS,
      )
      .await
      .expect("initial rumble should succeed");
    assert_eq!(
      state.lock().unwrap().rumble_log,
      vec![(id(7), 0x8000, 0x7fff, RUMBLE_DURATION_MS)],
      "initial non-zero command arms exactly once"
    );

    // Just before the refresh deadline: no re-arm.
    clock.advance_to(RUMBLE_KEEPALIVE_INTERVAL_MS - 1);
    handle.scan().await.expect("probe scan should succeed");
    assert_eq!(
      state.lock().unwrap().rumble_log.len(),
      1,
      "no re-arm before the deadline"
    );

    // Reaching the deadline triggers exactly one re-send with the same
    // parameters, comfortably before the finite arm lapses. A barrier rather
    // than a single scan: the refresh runs in the loop-top pass after the
    // scan's reply, so only the second command guarantees it has run.
    clock.advance_to(RUMBLE_KEEPALIVE_INTERVAL_MS);
    barrier(&handle).await;
    assert_eq!(
      state.lock().unwrap().rumble_log,
      vec![
        (id(7), 0x8000, 0x7fff, RUMBLE_DURATION_MS),
        (id(7), 0x8000, 0x7fff, RUMBLE_DURATION_MS),
      ]
    );

    // Not due again immediately: one probe wakes, no further re-arm.
    handle.scan().await.expect("probe scan should succeed");
    assert_eq!(state.lock().unwrap().rumble_log.len(), 2);
  }

  #[tokio::test]
  async fn sdl_task_refresh_stops_on_zero_close_removal_at_loop_level() {
    // (a) A zero-speed command stops refreshing.
    {
      let state = Arc::new(Mutex::new(FakeDriverState::default()));
      let clock = FakeClock::default();
      let handle = spawn_fake(state.clone(), clock.clone());
      let (opened, _) = handle.open(id(10)).await.expect("open should succeed");
      opened
        .set_rumble_state(
          SdlRumbleState {
            low: 100,
            high: 100,
            ..Default::default()
          },
          RUMBLE_DURATION_MS,
        )
        .await
        .expect("rumble should succeed");
      opened
        .set_rumble_state(SdlRumbleState::default(), RUMBLE_DURATION_MS)
        .await
        .expect("zero rumble should succeed");
      assert_eq!(state.lock().unwrap().rumble_log.len(), 2);
      for t in [
        RUMBLE_KEEPALIVE_INTERVAL_MS,
        RUMBLE_KEEPALIVE_INTERVAL_MS * 2,
        RUMBLE_KEEPALIVE_INTERVAL_MS * 3,
      ] {
        clock.advance_to(t);
        handle.scan().await.expect("probe scan should succeed");
      }
      assert_eq!(
        state.lock().unwrap().rumble_log.len(),
        2,
        "zero rumble must not be refreshed"
      );
    }

    // (b) Close stops refreshing.
    {
      let state = Arc::new(Mutex::new(FakeDriverState::default()));
      let clock = FakeClock::default();
      let handle = spawn_fake(state.clone(), clock.clone());
      let (opened, _) = handle.open(id(11)).await.expect("open should succeed");
      opened
        .set_rumble_state(
          SdlRumbleState {
            low: 100,
            high: 100,
            ..Default::default()
          },
          RUMBLE_DURATION_MS,
        )
        .await
        .expect("rumble should succeed");
      opened.close().await.expect("close should succeed");
      // Close while rumbling emits the zero-speed stop, then nothing more.
      clock.advance_to(RUMBLE_KEEPALIVE_INTERVAL_MS * 2);
      handle.scan().await.expect("probe scan should succeed");
      assert_eq!(
        state.lock().unwrap().rumble_log,
        vec![
          (id(11), 100, 100, RUMBLE_DURATION_MS),
          (id(11), 0, 0, RUMBLE_DURATION_MS),
        ],
        "closed gamepad must not be refreshed"
      );
    }

    // (c) Connected-state removal stops refreshing.
    {
      let state = Arc::new(Mutex::new(FakeDriverState::default()));
      let clock = FakeClock::default();
      let handle = spawn_fake(state.clone(), clock.clone());
      let (opened, _) = handle.open(id(12)).await.expect("open should succeed");
      opened
        .set_rumble_state(
          SdlRumbleState {
            low: 100,
            high: 100,
            ..Default::default()
          },
          RUMBLE_DURATION_MS,
        )
        .await
        .expect("rumble should succeed");
      state.lock().unwrap().connected.insert(id(12), false);
      // Keepalives re-arm while the pad still appears connected (each wake
      // at the 100ms cadence); the connected poll then observes the drop and
      // stop_and_drop emits the zero-speed stop, after which nothing further.
      clock.advance_to(RUMBLE_KEEPALIVE_INTERVAL_MS * 2);
      barrier(&handle).await;
      clock.advance_to(CONNECTED_POLL_INTERVAL_MS);
      barrier(&handle).await;
      assert_eq!(
        state.lock().unwrap().rumble_log,
        vec![
          (id(12), 100, 100, RUMBLE_DURATION_MS),
          (id(12), 100, 100, RUMBLE_DURATION_MS),
          (id(12), 0, 0, RUMBLE_DURATION_MS),
        ],
        "removed gamepad must not be refreshed"
      );
    }
  }

  // -------------------------------------------------------------------
  // Publication / init failure
  // -------------------------------------------------------------------

  #[test]
  fn sdl_task_init_failure_publishes_inert_state() {
    // Publication decision exercised on a LOCAL cell; the process-global
    // OnceLock is never touched by tests.
    let cell: OnceLock<PublishedSdlTask> = OnceLock::new();
    let published = publish_sdl_task(&cell, || Err(SdlTaskInitError("no SDL here".to_owned())));
    let err = published
      .as_ref()
      .expect_err("init failure must publish Err");
    assert_eq!(err.0, "no SDL here");

    // A backend over the inert publication reports cannot-scan and errors on
    // use.
    let leaked: &'static PublishedSdlTask = Box::leak(Box::new(cell.get().unwrap().clone()));
    let backend = SdlTaskBackend {
      publication: leaked,
    };
    assert!(!backend.initialized());

    // Second publication attempt returns the same, inert result (no retry).
    let again = publish_sdl_task(&cell, || panic!("must not be called again"));
    assert!(again.is_err());
  }

  #[tokio::test]
  async fn sdl_task_backend_over_inert_publication_errors_on_use() {
    let cell: &'static OnceLock<PublishedSdlTask> = Box::leak(Box::new(OnceLock::new()));
    publish_sdl_task(cell, || Err(SdlTaskInitError("nope".to_owned())));
    let backend = SdlTaskBackend {
      publication: cell.get().unwrap(),
    };
    assert!(!backend.initialized());
    let err = backend
      .gamepads()
      .await
      .expect_err("inert backend must not scan");
    assert!(matches!(err, SdlTaskError::Init(_)), "got {err:?}");
    let err = backend
      .open(id(1))
      .await
      .expect_err("inert backend must not open");
    assert!(matches!(err, SdlTaskError::Init(_)), "got {err:?}");
  }
}
