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
//! stable for the lifetime of a connection. Conversion to buttplug's string
//! address space (`sdl-gamepad-{instance_id}`) happens only at the
//! communication-manager boundary.
//!
//! Rumble is armed with a finite duration (the sdl3 crate documents that
//! `u32::MAX` overflows and ends the effect immediately) and refreshed by the
//! thread before expiry, so one-shot ScalarCmd commands hold indefinitely.

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

/// Elapsed time (ms) after which a still-active (non-zero) rumble is re-armed.
/// Comfortably before [`RUMBLE_DURATION_MS`] so the effect never lapses.
const RUMBLE_REFRESH_AFTER_MS: u64 = 50_000;

/// Interval (ms) at which open gamepads have their connected state polled.
const CONNECTED_POLL_INTERVAL_MS: u64 = 500;

/// Timeout (ms) of the command-receive wait; also the loop's wake granularity
/// for connected-poll and rumble-refresh checks.
const COMMAND_WAKE_MS: u64 = 100;

/// A gamepad discovered by a scan, with its SDL-reported name (or the
/// deterministic fallback name when the name lookup failed).
#[derive(Debug, Clone)]
pub(crate) struct SdlGamepadDesc {
  pub id: JoystickId,
  pub name: String,
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
pub(crate) trait DriverGamepad {
  fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String>;
  fn connected(&self) -> bool;
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
    reply: oneshot::Sender<Result<SdlOpenedGamepadHandle, SdlTaskError>>,
  },
  Rumble {
    id: JoystickId,
    generation: u64,
    low: u16,
    high: u16,
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

  pub(crate) async fn rumble(
    &self,
    low: u16,
    high: u16,
    duration_ms: u32,
  ) -> Result<(), SdlTaskError> {
    self
      .task
      .rumble(self.id, self.generation, low, high, duration_ms)
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
/// If all handles drop, the thread exits (which drops the SDL context). The
/// process-global publication keeps one handle alive for the process lifetime.
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

  pub(crate) async fn open(&self, id: JoystickId) -> Result<SdlOpenedGamepadHandle, SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Open { id, reply })
      .await?
  }

  pub(crate) async fn rumble(
    &self,
    id: JoystickId,
    generation: u64,
    low: u16,
    high: u16,
    duration: u32,
  ) -> Result<(), SdlTaskError> {
    self
      .send_and_await(|reply| SdlCommand::Rumble {
        id,
        generation,
        low,
        high,
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
  last_rumble: (u16, u16),
  last_set_at: u64,
}

/// Pure rumble-refresh decision: given the last accepted rumble command, when
/// it was armed, and the current time, decide whether it must be re-armed.
///
/// Zero-speed commands never refresh (the gamepad is stopped; letting the
/// effect lapse is exactly what we want). Non-zero commands re-arm after
/// [`RUMBLE_REFRESH_AFTER_MS`], safely before the finite arm duration lapses.
fn refresh_decision(last_rumble: (u16, u16), last_set_at: u64, now_ms: u64) -> Option<(u16, u16)> {
  if last_rumble == (0, 0) {
    return None;
  }
  if now_ms.saturating_sub(last_set_at) >= RUMBLE_REFRESH_AFTER_MS {
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
  if state.last_rumble != (0, 0) {
    let _ = state.pad.rumble(0, 0, RUMBLE_DURATION_MS);
  }
  mark_removed(state);
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
    let mut rumbles_to_refresh: Vec<(JoystickId, (u16, u16))> = Vec::new();
    for (id, state) in open_pads.iter() {
      if let Some(cmd) = refresh_decision(state.last_rumble, state.last_set_at, now) {
        rumbles_to_refresh.push((*id, cmd));
      }
    }
    for (id, (low, high)) in rumbles_to_refresh {
      let Some(state) = open_pads.get_mut(&id) else {
        continue;
      };
      match state.pad.rumble(low, high, RUMBLE_DURATION_MS) {
        Ok(()) => {
          state.last_set_at = now;
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
        SdlCommand::Scan { reply } => {
          let result = driver.enumerate().map_err(|e| {
            warn!("SDL gamepad enumeration failed: {}", e);
            SdlTaskError::Scan(e)
          });
          let reply_value = result.map(|ids| {
            ids
              .into_iter()
              .map(|id| {
                let name = match driver.name_for_id(id) {
                  Ok(name) => name,
                  Err(e) => {
                    // A failed name lookup never drops the device: log and
                    // fall back to a deterministic name.
                    warn!("SDL gamepad {} name lookup failed: {}", id.0, e);
                    format!("SDL Gamepad {}", id.0)
                  }
                };
                SdlGamepadDesc { id, name }
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
              next_generation += 1;
              let generation = next_generation;
              let (removed_tx, removed_rx) = watch::channel(false);
              open_pads.insert(
                id,
                OpenPadState {
                  pad,
                  generation,
                  removed_tx,
                  last_rumble: (0, 0),
                  last_set_at: now,
                },
              );
              let handle = SdlOpenedGamepadHandle {
                id,
                generation,
                task: task_tx.clone(),
                removed_rx,
              };
              if reply.send(Ok(handle)).is_err() {
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
        SdlCommand::Rumble {
          id,
          generation,
          low,
          high,
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
          let reply_value = state
            .pad
            .rumble(low, high, duration)
            .map_err(|e| SdlTaskError::Rumble(e));
          if reply_value.is_ok() {
            state.last_rumble = (low, high);
            state.last_set_at = clock.now_ms();
          }
          let _ = reply.send(reply_value);
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
        // All handles dropped; shut the thread (and SDL context) down.
        info!("SDL gamepad thread command channel closed; exiting.");
        for (_, state) in open_pads.drain() {
          stop_and_drop(state);
        }
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
  let (cmd_tx, cmd_rx) = mpsc::channel::<SdlCommand>();
  let (init_tx, init_rx) = mpsc::channel::<Result<(), SdlTaskInitError>>();
  let loop_tx = SdlTaskHandle {
    cmd_tx: cmd_tx.clone(),
  };
  std::thread::Builder::new()
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
  Ok(SdlTaskHandle { cmd_tx })
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
  fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String> {
    self
      .pad
      .set_rumble(low, high, duration_ms)
      .map_err(|e| e.to_string())
  }

  fn connected(&self) -> bool {
    self.pad.connected()
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
  async fn rumble(&self, low: u16, high: u16, duration_ms: u32) -> Result<(), SdlTaskError>;
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
  async fn open(&self, id: JoystickId) -> Result<Arc<dyn SdlOpenedGamepad>, SdlTaskError>;
}

/// Production opened-gamepad wrapper over the task handle.
#[derive(Debug)]
struct TaskOpenedGamepad {
  handle: SdlOpenedGamepadHandle,
}

#[async_trait]
impl SdlOpenedGamepad for TaskOpenedGamepad {
  async fn rumble(&self, low: u16, high: u16, duration_ms: u32) -> Result<(), SdlTaskError> {
    self.handle.rumble(low, high, duration_ms).await
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

  async fn open(&self, id: JoystickId) -> Result<Arc<dyn SdlOpenedGamepad>, SdlTaskError> {
    match self.publication {
      Ok(handle) => Ok(Arc::new(TaskOpenedGamepad {
        handle: handle.open(id).await?,
      })),
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
  }

  struct FakeDriver(Arc<Mutex<FakeDriverState>>);

  struct FakeGamepad {
    id: JoystickId,
    state: Arc<Mutex<FakeDriverState>>,
  }

  impl DriverGamepad for FakeGamepad {
    fn rumble(&mut self, low: u16, high: u16, duration_ms: u32) -> Result<(), String> {
      let mut state = self.state.lock().unwrap();
      if state.rumble_fail {
        return Err("rumble failed".to_owned());
      }
      state.rumble_log.push((self.id, low, high, duration_ms));
      Ok(())
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
        Ok(format!("SDL Fake Pad {}", id.0))
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

  #[tokio::test]
  async fn sdl_task_name_fallback_on_lookup_failure() {
    let state = Arc::new(Mutex::new(FakeDriverState {
      enumerate_ids: vec![id(2), id(3)],
      name_fail_ids: vec![id(3)],
      ..Default::default()
    }));
    let handle = spawn_fake(state, FakeClock::default());

    let descs = handle.scan().await.expect("scan should succeed");
    assert_eq!(descs.len(), 2);
    assert_eq!(descs[0].name, "SDL Fake Pad 2");
    // Failed name lookup falls back to the deterministic name; the device is
    // still returned.
    assert_eq!(descs[1].name, "SDL Gamepad 3");
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

    let opened = handle.open(id(4)).await.expect("open should succeed");
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
      .rumble(id(5), 0, 100, 100, RUMBLE_DURATION_MS)
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
    let opened = handle.open(id(13)).await.expect("open should succeed");
    opened
      .rumble(100, 100, RUMBLE_DURATION_MS)
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
    let opened = handle.open(id(14)).await.expect("open should succeed");
    opened
      .rumble(100, 100, RUMBLE_DURATION_MS)
      .await
      .expect("rumble should succeed");
    state.lock().unwrap().connected.insert(id(14), false);
    clock.advance_to(CONNECTED_POLL_INTERVAL_MS * 10);
    handle.scan().await.expect("probe scan should succeed");
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
  async fn sdl_task_stale_generation_is_inert() {
    let state = Arc::new(Mutex::new(FakeDriverState::default()));
    let handle = spawn_fake(state.clone(), FakeClock::default());

    // First lease: open, rumble, close (device stays connected).
    let stale = handle.open(id(15)).await.expect("open should succeed");
    stale
      .rumble(100, 100, RUMBLE_DURATION_MS)
      .await
      .expect("rumble should succeed");
    stale.close().await.expect("close should succeed");
    let log_len_after_first_lease = state.lock().unwrap().rumble_log.len();

    // Second lease for the same still-connected id.
    let fresh = handle.open(id(15)).await.expect("reopen should succeed");

    // Stale-handle rumble is rejected...
    let err = stale
      .rumble(1, 1, RUMBLE_DURATION_MS)
      .await
      .expect_err("stale rumble must fail");
    assert!(matches!(err, SdlTaskError::Removed(_)), "got {err:?}");
    // ...stale close is an Ok no-op that must NOT tear down the new lease...
    stale.close().await.expect("stale close is a no-op ok");
    // ...and the fresh lease still works.
    fresh
      .rumble(50, 50, RUMBLE_DURATION_MS)
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

    let opened = handle.open(id(6)).await.expect("open should succeed");
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
      .rumble(id(6), 0, 1, 1, RUMBLE_DURATION_MS)
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
      refresh_decision((100, 200), 1_000, 1_000 + RUMBLE_REFRESH_AFTER_MS - 1),
      None
    );
    // At the deadline: re-arm with the same speeds.
    assert_eq!(
      refresh_decision((100, 200), 1_000, 1_000 + RUMBLE_REFRESH_AFTER_MS),
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

    let opened = handle.open(id(7)).await.expect("open should succeed");
    opened
      .rumble(0x8000, 0x7fff, RUMBLE_DURATION_MS)
      .await
      .expect("initial rumble should succeed");
    assert_eq!(
      state.lock().unwrap().rumble_log,
      vec![(id(7), 0x8000, 0x7fff, RUMBLE_DURATION_MS)],
      "initial non-zero command arms exactly once"
    );

    // Just before the refresh deadline: no re-arm.
    clock.advance_to(RUMBLE_REFRESH_AFTER_MS - 1);
    handle.scan().await.expect("probe scan should succeed");
    assert_eq!(
      state.lock().unwrap().rumble_log.len(),
      1,
      "no re-arm before the deadline"
    );

    // Reaching the deadline triggers exactly one re-send with the same
    // parameters, comfortably before the finite arm lapses.
    clock.advance_to(RUMBLE_REFRESH_AFTER_MS);
    handle.scan().await.expect("probe scan should succeed");
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
      let opened = handle.open(id(10)).await.expect("open should succeed");
      opened
        .rumble(100, 100, RUMBLE_DURATION_MS)
        .await
        .expect("rumble should succeed");
      opened
        .rumble(0, 0, RUMBLE_DURATION_MS)
        .await
        .expect("zero rumble should succeed");
      assert_eq!(state.lock().unwrap().rumble_log.len(), 2);
      for t in [
        RUMBLE_REFRESH_AFTER_MS,
        RUMBLE_REFRESH_AFTER_MS * 2,
        RUMBLE_REFRESH_AFTER_MS * 3,
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
      let opened = handle.open(id(11)).await.expect("open should succeed");
      opened
        .rumble(100, 100, RUMBLE_DURATION_MS)
        .await
        .expect("rumble should succeed");
      opened.close().await.expect("close should succeed");
      // Close while rumbling emits the zero-speed stop, then nothing more.
      clock.advance_to(RUMBLE_REFRESH_AFTER_MS * 2);
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
      let opened = handle.open(id(12)).await.expect("open should succeed");
      opened
        .rumble(100, 100, RUMBLE_DURATION_MS)
        .await
        .expect("rumble should succeed");
      state.lock().unwrap().connected.insert(id(12), false);
      clock.advance_to(RUMBLE_REFRESH_AFTER_MS * 2);
      handle.scan().await.expect("probe scan should succeed");
      assert_eq!(
        state.lock().unwrap().rumble_log,
        vec![
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
