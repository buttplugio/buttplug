// Buttplug Rust Source Code File - See https://buttplug.io for more info.
// Licensed under the BSD 3-Clause license. See LICENSE in the project root.

//! FJB-03 host-side direction modes. The client continues sending unsigned
//! Oscillate strength; direction is NEVER inferred from amplitude or slope.

use super::hardware::{HardwareCommand, HardwareWriteCmd};
use buttplug_server_device_config::Endpoint;
use serde::{Deserialize, Serialize};
use std::{
  collections::{HashMap, VecDeque},
  sync::LazyLock,
  time::Duration,
};
use tokio::time::Instant;
use uuid::uuid;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MotionMode {
  #[default]
  Forward,
  Reverse,
  Alternate,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MotionSettings {
  pub mode: MotionMode,
  pub dwell_ms: u32,
  pub pause_ms: u32,
}

impl Default for MotionSettings {
  fn default() -> Self {
    Self {
      mode: MotionMode::Forward,
      dwell_ms: 1000,
      pause_ms: 150,
    }
  }
}

impl MotionSettings {
  pub fn validate(&self) -> Result<(), String> {
    if !(500..=10000).contains(&self.dwell_ms) || !(100..=1000).contains(&self.pause_ms) {
      return Err("FJB-03: dwell_ms must be 500..10000; pause_ms must be 100..1000".into());
    }
    Ok(())
  }
}

static SETTINGS: LazyLock<tokio::sync::watch::Sender<HashMap<String, MotionSettings>>> =
  LazyLock::new(|| tokio::sync::watch::channel(HashMap::new()).0);

/// Publish validated preferences without restarting connected devices.
pub fn configure(settings: HashMap<String, MotionSettings>) -> Result<(), String> {
  for value in settings.values() {
    value.validate()?;
  }
  SETTINGS.send_if_modified(|current| {
    if *current == settings {
      return false;
    }
    *current = settings;
    true
  });
  Ok(())
}

pub(super) fn settings_for(address: &str) -> MotionSettings {
  SETTINGS.borrow().get(address).copied().unwrap_or_default()
}

pub(super) fn subscribe() -> tokio::sync::watch::Receiver<HashMap<String, MotionSettings>> {
  SETTINGS.subscribe()
}

#[derive(Clone, Copy, Debug)]
enum Phase {
  Idle,
  Running(Instant),
  RunWrite,
  Pausing(Instant),
  PauseWrite,
}

pub(super) struct MotionController {
  settings: MotionSettings,
  phase: Phase,
  reverse: bool,
  target_reverse: Option<bool>,
  live: Option<HardwareWriteCmd>,
  vibration: Option<HardwareWriteCmd>,
}

impl MotionController {
  pub fn new(settings: MotionSettings) -> Self {
    Self {
      settings,
      phase: Phase::Idle,
      reverse: settings.mode == MotionMode::Reverse,
      target_reverse: None,
      live: None,
      vibration: None,
    }
  }

  pub fn deadline(&self) -> Option<Instant> {
    match self.phase {
      Phase::Running(t) | Phase::Pausing(t) => Some(t),
      _ => None,
    }
  }

  /// Called only between write batches. Never resurrect a stopped amplitude.
  pub fn reconfigure(&mut self, settings: MotionSettings, now: Instant) {
    if self.settings == settings {
      return;
    }
    self.settings = settings;
    self.target_reverse = None;
    if self.live.as_ref().is_some_and(|cmd| cmd.data()[2] != 0) {
      self.target_reverse = Some(settings.mode == MotionMode::Reverse);
      // Expire the old cycle now; the next packet is zero, then a fresh pause.
      self.phase = Phase::Running(now);
    } else {
      self.phase = Phase::Idle;
      self.reverse = settings.mode == MotionMode::Reverse;
    }
  }

  fn advance(&mut self, now: Instant) {
    match self.phase {
      Phase::Running(t) if now >= t => self.phase = Phase::PauseWrite,
      Phase::Pausing(t) if now >= t => {
        self.reverse = self.target_reverse.take().unwrap_or(!self.reverse);
        self.phase = Phase::RunWrite;
      }
      _ => {}
    }
  }

  /// Start timing only AFTER the whole write batch returns, especially the
  /// zero-before-reversal. This is OS submission timing, not a motor ACK.
  pub fn written(&mut self, now: Instant) {
    match self.phase {
      Phase::RunWrite => {
        self.phase = if self.settings.mode == MotionMode::Alternate {
          Phase::Running(now + Duration::from_millis(self.settings.dwell_ms as u64))
        } else {
          Phase::Idle
        };
      }
      Phase::PauseWrite => {
        self.phase = Phase::Pausing(now + Duration::from_millis(self.settings.pause_ms as u64))
      }
      _ => {}
    }
  }

  fn render(&self, cmd: &HardwareWriteCmd) -> HardwareCommand {
    let mut data = cmd.data().clone();
    let amplitude = data[2].min(20);
    data[2] = if amplitude == 0 || matches!(self.phase, Phase::PauseWrite | Phase::Pausing(_)) {
      0
    } else if self.reverse {
      20 + amplitude
    } else {
      amplitude
    };
    data[5] = data[..5].iter().copied().fold(0u8, u8::wrapping_add);
    HardwareWriteCmd::new(
      &cmd.command_id().iter().copied().collect::<Vec<_>>(),
      cmd.endpoint(),
      data,
      cmd.write_with_response(),
    )
    .into()
  }

  pub fn transform(
    &mut self,
    commands: VecDeque<HardwareCommand>,
    now: Instant,
  ) -> VecDeque<HardwareCommand> {
    commands
      .into_iter()
      .map(|command| {
        if let HardwareCommand::Write(cmd) = &command {
          if cmd.endpoint() == Endpoint::Tx
            && cmd.data().len() == 6
            && cmd.data()[..2] == [0x35, 0x12]
          {
            self.live = Some(cmd.clone());
            if cmd.data()[2] == 0 {
              self.phase = Phase::Idle;
              self.target_reverse = None;
              self.reverse = self.settings.mode == MotionMode::Reverse;
            } else if self.settings.mode == MotionMode::Alternate {
              if matches!(self.phase, Phase::Idle) {
                self.phase = Phase::RunWrite;
              }
            }
            self.advance(now);
            return self.render(cmd);
          }
          if cmd.endpoint() == Endpoint::Tx
            && cmd.data().len() == 5
            && cmd.data()[..3] == [0x35, 0x11, 4]
          {
            self.vibration = (cmd.data()[3] != 0).then(|| cmd.clone());
          }
        }
        command
      })
      .collect()
  }

  pub fn tick(&mut self, now: Instant) -> VecDeque<HardwareCommand> {
    if !self.deadline().is_some_and(|t| now >= t) {
      return VecDeque::new();
    }
    self.advance(now);
    let mut result = VecDeque::new();
    if let Some(live) = &self.live {
      result.push_back(self.render(live));
      // Live A/B packets can cancel fixed C mode, including timer-generated
      // packets. Keep suction unchanged and reapply active vibration.
      if let Some(vibration) = &self.vibration {
        result.push_back(vibration.clone().into());
      }
    }
    result
  }

  pub fn stop_all(&mut self) -> VecDeque<HardwareCommand> {
    self.phase = Phase::Idle;
    self.target_reverse = None;
    self.live = None;
    self.vibration = None;
    VecDeque::from([
      HardwareWriteCmd::new(
        &[uuid!("b5ccbc68-d970-4e91-b0fa-7ebf74efbb91")],
        Endpoint::Tx,
        vec![0x35, 0x11, 4, 0, 0x4a],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &[uuid!("d5987116-2fba-4c30-a7aa-ef567a3bf35d")],
        Endpoint::Tx,
        vec![0x35, 0x12, 0, 0, 0, 0x47],
        false,
      )
      .into(),
    ])
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  fn input(a: u8, b: u8) -> VecDeque<HardwareCommand> {
    VecDeque::from([HardwareWriteCmd::new(
      &[],
      Endpoint::Tx,
      vec![0x35, 0x12, a, b, 0, 0x47 + a + b],
      false,
    )
    .into()])
  }
  fn packet(commands: &VecDeque<HardwareCommand>) -> Vec<u8> {
    match &commands[0] {
      HardwareCommand::Write(cmd) => cmd.data().clone(),
      _ => panic!(),
    }
  }
  fn alternate() -> MotionController {
    MotionController::new(MotionSettings {
      mode: MotionMode::Alternate,
      ..Default::default()
    })
  }

  #[test]
  fn both_fixed_directions_use_equal_amplitude_and_zero_is_always_zero() {
    for mode in [MotionMode::Forward, MotionMode::Reverse] {
      let mut c = MotionController::new(MotionSettings {
        mode,
        ..Default::default()
      });
      for a in 0..=20 {
        let p = packet(&c.transform(input(a, 7), Instant::now()));
        assert_eq!(
          p[2],
          if mode == MotionMode::Reverse && a > 0 {
            a + 20
          } else {
            a
          }
        );
        assert_eq!(p[3], 7);
        assert_eq!(p[5], p[..5].iter().copied().fold(0u8, u8::wrapping_add));
        assert!(c.deadline().is_none());
      }
    }
  }
  #[test]
  fn fixed_input_alternates_without_more_client_commands() {
    let mut c = alternate();
    let t = Instant::now();
    assert_eq!(packet(&c.transform(input(10, 8), t))[2], 10);
    assert!(c.deadline().is_none());
    c.written(t);
    assert!(c.tick(t + Duration::from_millis(999)).is_empty());
    assert_eq!(packet(&c.tick(t + Duration::from_secs(1)))[2], 0);
    assert!(c.deadline().is_none());
    c.written(t + Duration::from_secs(2)); // slow zero submission: pause starts here
    assert!(c.tick(t + Duration::from_millis(2149)).is_empty());
    let p = packet(&c.tick(t + Duration::from_millis(2150)));
    assert_eq!((p[2], p[3]), (30, 8));
    c.written(t + Duration::from_millis(2150));
    assert_eq!(packet(&c.tick(t + Duration::from_millis(3150)))[2], 0);
    c.written(t + Duration::from_millis(3150));
    assert_eq!(packet(&c.tick(t + Duration::from_millis(3300)))[2], 10);
  }
  #[test]
  fn newest_amplitude_during_pause_is_used_without_shortening_pause() {
    let mut c = alternate();
    let t = Instant::now();
    c.transform(input(10, 0), t);
    c.written(t);
    c.tick(t + Duration::from_secs(1));
    c.written(t + Duration::from_secs(1));
    assert_eq!(
      packet(&c.transform(input(16, 9), t + Duration::from_millis(1050)))[2],
      0
    );
    c.written(t + Duration::from_millis(1050));
    assert_eq!(c.deadline(), Some(t + Duration::from_millis(1150)));
    assert_eq!(packet(&c.tick(t + Duration::from_millis(1150)))[2], 36);
  }
  #[test]
  fn zero_cancels_every_phase_and_never_restarts_from_a_timer() {
    for phase in 0..3 {
      let mut c = alternate();
      let t = Instant::now();
      c.transform(input(10, 0), t);
      c.written(t);
      if phase > 0 {
        c.tick(t + Duration::from_secs(1));
      }
      if phase > 1 {
        c.written(t + Duration::from_secs(1));
      }
      assert_eq!(
        packet(&c.transform(input(0, 0), t + Duration::from_secs(2)))[2],
        0
      );
      c.written(t + Duration::from_secs(2));
      assert!(c.tick(t + Duration::from_secs(30)).is_empty());
      assert_eq!(
        packet(&c.transform(input(8, 0), t + Duration::from_secs(31)))[2],
        8
      );
    }
  }
  #[test]
  fn continuous_updates_do_not_reset_dwell_and_timer_preserves_vibration() {
    let mut c = alternate();
    let t = Instant::now();
    c.transform(input(10, 5), t);
    c.written(t);
    c.transform(
      VecDeque::from([HardwareWriteCmd::new(
        &[],
        Endpoint::Tx,
        vec![0x35, 0x11, 4, 3, 0x4d],
        false,
      )
      .into()]),
      t,
    );
    c.transform(input(15, 5), t + Duration::from_millis(900));
    c.written(t + Duration::from_millis(900));
    assert_eq!(c.deadline(), Some(t + Duration::from_secs(1)));
    let p = c.tick(t + Duration::from_secs(1));
    assert_eq!(p.len(), 2);
    assert_eq!(packet(&p)[3], 5);
    let stopped = c.stop_all();
    assert_eq!(stopped.len(), 2);
    assert!(c.tick(t + Duration::from_secs(20)).is_empty());
  }
  #[test]
  fn live_switch_replaces_cycle_and_waits_for_zero_submission() {
    let mut c = alternate();
    let t = Instant::now();
    c.transform(input(10, 8), t);
    c.written(t);
    c.reconfigure(
      MotionSettings {
        mode: MotionMode::Reverse,
        ..Default::default()
      },
      t,
    );
    assert_eq!(packet(&c.tick(t))[2], 0);
    assert!(c.deadline().is_none());
    c.written(t + Duration::from_secs(2));
    assert_eq!(
      packet(&c.transform(input(15, 8), t + Duration::from_millis(2100)))[2],
      0
    );
    c.written(t + Duration::from_millis(2100));
    let p = packet(&c.tick(t + Duration::from_millis(2150)));
    assert_eq!((p[2], p[3]), (35, 8));
    c.written(t + Duration::from_millis(2150));
    assert!(c.deadline().is_none());
    assert!(c.tick(t + Duration::from_secs(30)).is_empty());
    c.reconfigure(MotionSettings::default(), t + Duration::from_secs(31));
    assert_eq!(packet(&c.tick(t + Duration::from_secs(31)))[2], 0);
    c.written(t + Duration::from_secs(31));
    assert_eq!(packet(&c.tick(t + Duration::from_millis(31150)))[2], 15);
  }

  #[test]
  fn live_settings_never_restart_zero_or_stop_and_latest_mode_wins() {
    let mut c = alternate();
    let t = Instant::now();
    c.reconfigure(MotionSettings::default(), t);
    assert!(c.tick(t).is_empty());
    c.transform(input(10, 4), t);
    c.reconfigure(
      MotionSettings {
        mode: MotionMode::Reverse,
        ..Default::default()
      },
      t,
    );
    assert_eq!(packet(&c.tick(t))[2], 0);
    c.written(t);
    c.reconfigure(
      MotionSettings {
        mode: MotionMode::Alternate,
        ..Default::default()
      },
      t,
    );
    assert_eq!(packet(&c.tick(t))[2], 0);
    c.written(t);
    assert_eq!(packet(&c.tick(t + Duration::from_millis(150)))[2], 10);
    c.written(t + Duration::from_millis(150));
    assert!(c.deadline().is_some());
    c.reconfigure(MotionSettings::default(), t);
    c.transform(input(0, 4), t);
    c.written(t);
    assert!(c.tick(t + Duration::from_secs(30)).is_empty());
    c.stop_all();
    c.reconfigure(
      MotionSettings {
        mode: MotionMode::Reverse,
        ..Default::default()
      },
      t,
    );
    assert!(c.tick(t + Duration::from_secs(30)).is_empty());
  }

  #[test]
  fn invalid_settings_are_rejected_not_silently_applied() {
    assert!(MotionSettings::default().validate().is_ok());
    for dwell_ms in [0, 499, 10001, u32::MAX] {
      assert!(
        MotionSettings {
          dwell_ms,
          ..Default::default()
        }
        .validate()
        .is_err()
      );
    }
    for pause_ms in [0, 99, 1001] {
      assert!(
        MotionSettings {
          pause_ms,
          ..Default::default()
        }
        .validate()
        .is_err()
      );
    }
    assert!(
      serde_json::from_str::<MotionSettings>(r#"{"mode":"wrong","dwell_ms":1000,"pause_ms":150}"#)
        .is_err()
    );
  }
}
