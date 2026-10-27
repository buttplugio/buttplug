// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareReadCmd, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolIdentifier, ProtocolInitializer},
};
use async_trait::async_trait;
use buttplug_core::{errors::ButtplugDeviceError, message::OutputType};
use buttplug_server_device_config::{
  Endpoint,
  ProtocolCommunicationSpecifier,
  ServerDeviceDefinition,
  UserDeviceIdentifier,
};
use instant::Instant;
use std::{
  sync::{
    Arc,
    Mutex,
    atomic::{AtomicBool, AtomicU8, Ordering},
  },
  time::Duration,
};
use uuid::{Uuid, uuid};

const HISMITH_MINI_PROTOCOL_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");
const HISMITH_MINI_ROTATE_DIRECTIOM_UUID: Uuid = uuid!("94befc1a-9859-4bf6-99ee-5678c89237a7");

// Servo machines (HISMITH S2 / Servo) expose these Position features, by feature index. The
// opcodes come from the official app's CC command set.
const POSITION_FEATURE_STROKE_START: u32 = 1; // opcode 0x07
const POSITION_FEATURE_STROKE_END: u32 = 2; // opcode 0x08
const POSITION_FEATURE_DIRECT: u32 = 3; // opcode 0x0A, move to this depth now
const POSITION_FEATURE_SMOOTHNESS: u32 = 4; // opcode 0x09, 0-10
const POSITION_FEATURE_RUN_MODE: u32 = 5; // opcode 0x01, 0 = auto (0xA0), 1 = manual (0xA1)
const POSITION_FEATURE_PRESET_MODE: u32 = 6; // opcode 0x04, built-in modes 1-8, 0 = none

// The official app never lets the stroke end get closer than this to the stroke start.
const MIN_STROKE_GAP: u8 = 10;

// Simple two-slider S2 (speed + depth): the single Position feature at index 1 is "depth". Depth
// 0..100 slides a stroke window of DEPTH_WINDOW_WIDTH along the machine's travel (0 = shallow
// end, 100 = deep end): 0 -> 0..40, 50 -> 30..70, 100 -> 60..100.
const POSITION_FEATURE_DEPTH: u32 = 1;
const DEPTH_WINDOW_WIDTH: u8 = 40;

// Lube Launcher / "PiuPiu" (model 4002): one CC 0B 01 0C frame is one squirt. The app repeats the
// frame every 100 ms while its button is held and never sends the "off" frame, so we only fire
// for a non-zero level and not more often than that.
const SPRAY_FRAME: [u8; 4] = [0xCC, 0x0B, 0x01, 0x0C];
const SPRAY_MIN_INTERVAL: Duration = Duration::from_millis(100);
// On the PiuPiu, the Position feature at index 0 is the spray trigger. (Servo machines use
// index 0 for speed, which is not a Position feature.)
const POSITION_FEATURE_SPRAY: u32 = 0;

pub mod setup {
  use crate::device::protocol::{ProtocolIdentifier, ProtocolIdentifierFactory};
  #[derive(Default)]
  pub struct HismithMiniIdentifierFactory {}

  impl ProtocolIdentifierFactory for HismithMiniIdentifierFactory {
    fn identifier(&self) -> &str {
      "hismith-mini"
    }

    fn create(&self) -> Box<dyn ProtocolIdentifier> {
      Box::new(super::HismithMiniIdentifier::default())
    }
  }
}

#[derive(Default)]
pub struct HismithMiniIdentifier {}

#[async_trait]
impl ProtocolIdentifier for HismithMiniIdentifier {
  async fn identify(
    &mut self,
    hardware: Arc<Hardware>,
    _: ProtocolCommunicationSpecifier,
  ) -> Result<(UserDeviceIdentifier, Box<dyn ProtocolInitializer>), ButtplugDeviceError> {
    let result = hardware
      .read_value(&HardwareReadCmd::new(
        HISMITH_MINI_PROTOCOL_UUID,
        Endpoint::RxBLEModel,
        128,
        500,
      ))
      .await?;

    let identifier = result
      .data()
      .iter()
      .map(|b| format!("{b:02x}"))
      .collect::<String>();
    info!("Hismith Device Identifier: {}", identifier);

    Ok((
      UserDeviceIdentifier::new(hardware.address(), "hismith-mini", &Some(identifier)),
      Box::new(HismithMiniInitializer::default()),
    ))
  }
}

#[derive(Default)]
pub struct HismithMiniInitializer {}

#[async_trait]
impl ProtocolInitializer for HismithMiniInitializer {
  async fn initialize(
    &mut self,
    _: Arc<Hardware>,
    device_definition: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    // Depth-window mode: a machine with a speed (Oscillate) feature and exactly one Position
    // feature (the "depth" slider). Definitions with the full 0x07/0x08/0x0A feature set keep
    // the raw per-opcode behaviour.
    let position_features = device_definition
      .features()
      .values()
      .filter(|x| x.contains_output(OutputType::Position))
      .count();
    let has_oscillate = device_definition
      .features()
      .values()
      .any(|x| x.contains_output(OutputType::Oscillate));
    Ok(Arc::new(HismithMini {
      stroke_start: AtomicU8::new(0),
      stroke_end: AtomicU8::new(100),
      depth_axis: has_oscillate && position_features == 1,
      depth_sent: AtomicBool::new(false),
      last_spray: Mutex::new(None),
      dual_vibe: device_definition
        .features()
        .values()
        .filter(|x| x.contains_output(OutputType::Vibrate))
        .count()
        >= 2,
      second_constrict: device_definition
        .features()
        .values()
        .position(|x| x.contains_output(OutputType::Constrict))
        .unwrap_or(0)
        == 1,
    }))
  }
}

pub struct HismithMini {
  dual_vibe: bool,
  second_constrict: bool,
  // Last stroke window we sent (the machine boots with 0-100 unless the app changed it). Used to
  // keep end >= start + MIN_STROKE_GAP like the official app does.
  stroke_start: AtomicU8,
  stroke_end: AtomicU8,
  // True for the two-slider S2 definition; see POSITION_FEATURE_DEPTH.
  depth_axis: bool,
  // The machine's real window is unknown until the first depth command, so that one always writes
  // both limits; later ones skip identical windows.
  depth_sent: AtomicBool,
  last_spray: Mutex<Option<Instant>>,
}

impl HismithMini {
  // Slide the stroke window for a depth value 0..100. The two limits are written in the order that
  // keeps the window valid while it moves: deeper = end first, shallower = start first.
  fn depth_window(&self, feature_id: Uuid, depth: u32) -> Vec<HardwareCommand> {
    let span = (100 - DEPTH_WINDOW_WIDTH) as u32;
    let start = ((depth.min(100) * span + 50) / 100) as u8;
    let end = start + DEPTH_WINDOW_WIDTH;
    let old_start = self.stroke_start.load(Ordering::Relaxed);
    let first = !self.depth_sent.swap(true, Ordering::Relaxed);
    if !first && start == old_start {
      return vec![];
    }
    self.stroke_start.store(start, Ordering::Relaxed);
    self.stroke_end.store(end, Ordering::Relaxed);
    let write = |op: u8, value: u8| -> HardwareCommand {
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, op, value, value + op],
        false,
      )
      .into()
    };
    if start >= old_start {
      vec![write(0x08, end), write(0x07, start)]
    } else {
      vec![write(0x07, start), write(0x08, end)]
    }
  }

  fn spray(&self, feature_id: Uuid, level: u32) -> Vec<HardwareCommand> {
    if level == 0 {
      return vec![];
    }
    let mut last_spray = self.last_spray.lock().unwrap();
    let now = Instant::now();
    if let Some(last) = *last_spray
      && now.duration_since(last) < SPRAY_MIN_INTERVAL
    {
      return vec![];
    }
    *last_spray = Some(now);
    vec![HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, SPRAY_FRAME.to_vec(), false).into()]
  }
}

impl Default for HismithMini {
  fn default() -> Self {
    Self {
      dual_vibe: false,
      second_constrict: false,
      stroke_start: AtomicU8::new(0),
      stroke_end: AtomicU8::new(100),
      depth_axis: false,
      depth_sent: AtomicBool::new(false),
      last_spray: Mutex::new(None),
    }
  }
}

impl ProtocolHandler for HismithMini {
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = 0x03;
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_vibrate_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if !self.dual_vibe || feature_index == 1 {
      0x05
    } else {
      0x03
    };
    let speed: u8 = speed as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let idx: u8 = if self.second_constrict { 0x05 } else { 0x03 };
    let speed: u8 = level as u8;

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, speed, speed + idx],
        false,
      )
      .into(),
    ])
  }

  fn handle_output_rotate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![
          0xCC,
          0x03,
          speed.unsigned_abs() as u8,
          speed.unsigned_abs() as u8 + 3,
        ],
        false,
      )
      .into(),
      HardwareWriteCmd::new(
        &[HISMITH_MINI_ROTATE_DIRECTIOM_UUID],
        Endpoint::Tx,
        vec![
          0xCC,
          0x01,
          if speed >= 0 { 0xc0 } else { 0xc1 },
          if speed >= 0 { 0xc1 } else { 0xc2 },
        ],
        false,
      )
      .into(),
    ])
  }

  // Depth/stroke control of the servo machines (HISMITH S2 / Servo), values are 0-100 percent of
  // the machine's travel:
  //   index 1 -> stroke start (inner limit), 0x07
  //   index 2 -> stroke end (outer limit), 0x08
  //   index 3 -> direct position, move to this depth now, 0x0A
  //   index 4 -> smoothness, 0x09 (0-10)
  //   index 5 -> run mode, 0x01: 0 = auto (machine strokes on its own), 1 = manual (follows
  //              the direct position)
  //   index 6 -> built-in preset mode 1-8, 0x04 (0 sends nothing; a speed command leaves the mode)
  // The stroke window is kept at least MIN_STROKE_GAP wide, as the official app does.
  fn handle_output_position_cmd(
    &self,
    feature_index: u32,
    feature_id: Uuid,
    position: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    if feature_index == POSITION_FEATURE_SPRAY {
      return Ok(self.spray(feature_id, position));
    }
    if self.depth_axis && feature_index == POSITION_FEATURE_DEPTH {
      return Ok(self.depth_window(feature_id, position));
    }

    // The run mode command is the only one with a fixed value pair: CC 01 A0 / CC 01 A1.
    if feature_index == POSITION_FEATURE_RUN_MODE {
      let mode: u8 = if position == 0 { 0xA0 } else { 0xA1 };
      return Ok(vec![
        HardwareWriteCmd::new(
          &[feature_id],
          Endpoint::Tx,
          vec![0xCC, 0x01, mode, mode + 1],
          false,
        )
        .into(),
      ]);
    }
    if feature_index == POSITION_FEATURE_PRESET_MODE && position == 0 {
      return Ok(vec![]);
    }

    let (idx, value): (u8, u8) = match feature_index {
      POSITION_FEATURE_PRESET_MODE => (0x04, position.min(8) as u8),
      POSITION_FEATURE_STROKE_START => {
        let end = self.stroke_end.load(Ordering::Relaxed);
        let start = (position.min(100) as u8).min(end.saturating_sub(MIN_STROKE_GAP));
        self.stroke_start.store(start, Ordering::Relaxed);
        (0x07, start)
      }
      POSITION_FEATURE_DIRECT => (0x0A, position.min(100) as u8),
      POSITION_FEATURE_SMOOTHNESS => (0x09, position.min(10) as u8),
      // anything unexpected is treated as the stroke end as well
      POSITION_FEATURE_STROKE_END | _ => {
        let start = self.stroke_start.load(Ordering::Relaxed);
        let end = (position.min(100) as u8).max((start + MIN_STROKE_GAP).min(100));
        self.stroke_end.store(end, Ordering::Relaxed);
        (0x08, end)
      }
    };

    Ok(vec![
      HardwareWriteCmd::new(
        &[feature_id],
        Endpoint::Tx,
        vec![0xCC, idx, value, value + idx],
        false,
      )
      .into(),
    ])
  }

  // Lube Launcher: any non-zero level squirts once. Level 0 (also what a Stop sends) does nothing.
  fn handle_output_spray_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    level: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(self.spray(feature_id, level))
  }
}

#[cfg(test)]
mod test {
  use super::*;

  fn written(cmds: Vec<HardwareCommand>) -> Vec<u8> {
    match cmds.into_iter().next().unwrap() {
      HardwareCommand::Write(w) => w.data().clone(),
      _ => panic!("expected a write command"),
    }
  }

  #[test]
  fn stroke_start_uses_opcode_07() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(1, Uuid::nil(), 20)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x07, 0x14, 0x1B]);
  }

  #[test]
  fn stroke_end_uses_opcode_08() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 80)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x50, 0x58]);
  }

  #[test]
  fn stroke_position_is_clamped_to_100() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 250)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x64, 0x6C]);
  }

  #[test]
  fn speed_command_is_unchanged() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_oscillate_cmd(0, Uuid::nil(), 50)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x03, 0x32, 0x35]);
  }

  #[test]
  fn direct_position_uses_opcode_0a() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(3, Uuid::nil(), 50)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x0A, 0x32, 0x3C]);
  }

  #[test]
  fn smoothness_uses_opcode_09_and_clamps_to_10() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(4, Uuid::nil(), 7)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x09, 0x07, 0x10]);
    let cmds = handler
      .handle_output_position_cmd(4, Uuid::nil(), 99)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x09, 0x0A, 0x13]);
  }

  #[test]
  fn stroke_start_cannot_pass_end_minus_gap() {
    let handler = HismithMini::default();
    // end defaults to 100, so start is limited to 90
    let cmds = handler
      .handle_output_position_cmd(1, Uuid::nil(), 100)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x07, 0x5A, 0x61]);
  }

  #[test]
  fn stroke_end_cannot_pass_start_plus_gap() {
    let handler = HismithMini::default();
    handler
      .handle_output_position_cmd(1, Uuid::nil(), 40)
      .unwrap();
    // start is 40, so end is limited to 50
    let cmds = handler
      .handle_output_position_cmd(2, Uuid::nil(), 0)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x08, 0x32, 0x3A]);
  }

  #[test]
  fn run_mode_switches_between_auto_and_manual() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(5, Uuid::nil(), 0)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x01, 0xA0, 0xA1]);
    let cmds = handler
      .handle_output_position_cmd(5, Uuid::nil(), 1)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x01, 0xA1, 0xA2]);
  }

  #[test]
  fn preset_modes_one_to_eight_match_the_app() {
    // The app's product catalog for the S2 lists CC040105 ... CC04080C.
    let expected: [[u8; 4]; 8] = [
      [0xCC, 0x04, 0x01, 0x05],
      [0xCC, 0x04, 0x02, 0x06],
      [0xCC, 0x04, 0x03, 0x07],
      [0xCC, 0x04, 0x04, 0x08],
      [0xCC, 0x04, 0x05, 0x09],
      [0xCC, 0x04, 0x06, 0x0A],
      [0xCC, 0x04, 0x07, 0x0B],
      [0xCC, 0x04, 0x08, 0x0C],
    ];
    let handler = HismithMini::default();
    for (i, frame) in expected.iter().enumerate() {
      let cmds = handler
        .handle_output_position_cmd(6, Uuid::nil(), i as u32 + 1)
        .unwrap();
      assert_eq!(&written(cmds), frame);
    }
  }

  #[test]
  fn preset_mode_zero_sends_nothing() {
    let handler = HismithMini::default();
    assert!(
      handler
        .handle_output_position_cmd(6, Uuid::nil(), 0)
        .unwrap()
        .is_empty()
    );
  }

  #[test]
  fn spray_fires_the_app_frame_for_nonzero_level() {
    let handler = HismithMini::default();
    let cmds = handler.handle_output_spray_cmd(0, Uuid::nil(), 1).unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x0B, 0x01, 0x0C]);
  }

  #[test]
  fn spray_level_zero_does_nothing() {
    // Stop sends level 0; it must not squirt.
    let handler = HismithMini::default();
    assert!(handler.handle_output_spray_cmd(0, Uuid::nil(), 0).unwrap().is_empty());
  }

  #[test]
  fn spray_is_rate_limited_to_the_apps_100ms() {
    let handler = HismithMini::default();
    assert_eq!(
      handler.handle_output_spray_cmd(0, Uuid::nil(), 1).unwrap().len(),
      1
    );
    assert!(handler.handle_output_spray_cmd(0, Uuid::nil(), 1).unwrap().is_empty());
    std::thread::sleep(SPRAY_MIN_INTERVAL + Duration::from_millis(20));
    assert_eq!(
      handler.handle_output_spray_cmd(0, Uuid::nil(), 1).unwrap().len(),
      1
    );
  }

  #[test]
  fn spray_through_the_position_feature_for_v3_clients() {
    let handler = HismithMini::default();
    let cmds = handler
      .handle_output_position_cmd(0, Uuid::nil(), 1)
      .unwrap();
    assert_eq!(written(cmds), vec![0xCC, 0x0B, 0x01, 0x0C]);
    let handler = HismithMini::default();
    assert!(
      handler
        .handle_output_position_cmd(0, Uuid::nil(), 0)
        .unwrap()
        .is_empty()
    );
  }

  fn writes(cmds: Vec<HardwareCommand>) -> Vec<Vec<u8>> {
    cmds
      .into_iter()
      .map(|c| match c {
        HardwareCommand::Write(w) => w.data().clone(),
        _ => panic!("expected a write command"),
      })
      .collect()
  }

  fn depth_handler() -> HismithMini {
    HismithMini {
      depth_axis: true,
      ..HismithMini::default()
    }
  }

  #[test]
  fn depth_slides_a_40_wide_window_deeper_end_first() {
    let handler = depth_handler();
    // deepest: window 60..100, moving deeper -> end first, then start
    let cmds = handler
      .handle_output_position_cmd(1, Uuid::nil(), 100)
      .unwrap();
    assert_eq!(
      writes(cmds),
      vec![vec![0xCC, 0x08, 0x64, 0x6C], vec![0xCC, 0x07, 0x3C, 0x43]]
    );
  }

  #[test]
  fn depth_moving_shallower_writes_start_first() {
    let handler = depth_handler();
    handler.handle_output_position_cmd(1, Uuid::nil(), 100).unwrap();
    // shallowest: window 0..40, moving shallower -> start first, then end
    let cmds = handler.handle_output_position_cmd(1, Uuid::nil(), 0).unwrap();
    assert_eq!(
      writes(cmds),
      vec![vec![0xCC, 0x07, 0x00, 0x07], vec![0xCC, 0x08, 0x28, 0x30]]
    );
  }

  #[test]
  fn depth_middle_is_30_to_70() {
    let handler = depth_handler();
    let cmds = handler.handle_output_position_cmd(1, Uuid::nil(), 50).unwrap();
    assert_eq!(
      writes(cmds),
      vec![vec![0xCC, 0x08, 0x46, 0x4E], vec![0xCC, 0x07, 0x1E, 0x25]]
    );
  }

  #[test]
  fn depth_repeat_is_deduplicated_but_first_always_writes() {
    let handler = depth_handler();
    // the machine's real window is unknown at first: even depth 0 (0..40) writes both limits
    assert_eq!(
      writes(handler.handle_output_position_cmd(1, Uuid::nil(), 0).unwrap()).len(),
      2
    );
    assert!(handler.handle_output_position_cmd(1, Uuid::nil(), 0).unwrap().is_empty());
  }

  #[test]
  fn depth_window_never_inverts() {
    let handler = depth_handler();
    for depth in [0u32, 10, 33, 50, 77, 100, 55, 5, 100, 0] {
      let cmds = handler.handle_output_position_cmd(1, Uuid::nil(), depth).unwrap();
      let start = handler.stroke_start.load(Ordering::Relaxed);
      let end = handler.stroke_end.load(Ordering::Relaxed);
      assert_eq!(end - start, DEPTH_WINDOW_WIDTH);
      assert!(end <= 100);
      let _ = cmds;
    }
  }
}
