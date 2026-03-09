// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.
use crate::device::{
  hardware::{Hardware, HardwareCommand, HardwareEvent, HardwareSubscribeCmd, HardwareWriteCmd},
  protocol::{
    ProtocolHandler, ProtocolIdentifier, ProtocolInitializer, generic_protocol_initializer_setup,
  },
};
use async_trait::async_trait;
use buttplug_core::{
  errors::ButtplugDeviceError,
  util::{async_manager, sleep},
};
use buttplug_server_device_config::{
  Endpoint, ProtocolCommunicationSpecifier, ServerDeviceDefinition, UserDeviceIdentifier,
};
use futures::FutureExt;
use std::{
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, Ordering},
  },
  time::Duration,
};
use tokio::select;
use uuid::{Uuid, uuid};

const FMACHINE_PROTOCOL_UUID: Uuid = uuid!("0000fff0-0000-1000-8000-00805f9b34fb");

// Device registers 1 speed step per 200ms internally.
const FMACHINE_COMMAND_TIMEOUT_MS: u64 = 200;

// Init normalization cadence: matches official app's remote-start speed-down sequence.
const FMACHINE_INIT_STEP_MS: u64 = 60;

// 55 down-presses is enough to bring the device from its maximum speed down to 1.
// Speed Down cannot reduce the device's remembered speed below 1.
const FMACHINE_INIT_STEPS: u8 = 55;

// Command bytes for BLE packets. Full packet built by make_cmd().
const CMD_ON_OFF_PRESS: u8 = 0x01;
const CMD_ON_OFF_RELEASE: u8 = 0x02;
const CMD_SPEED_RELEASE: u8 = 0x03;
// No 0x04 Command byte
const CMD_SPEED_UP: u8 = 0x05;
const CMD_SPEED_DOWN: u8 = 0x06;
const CMD_SECONDARY_UP: u8 = 0x07;
const CMD_SECONDARY_DOWN: u8 = 0x08;
const CMD_SECONDARY_RELEASE: u8 = 0x09;

generic_protocol_initializer_setup!(FMachine, "fmachine");

/// Compute the non-standard CRC-8 used by the FMachine BLE protocol.
///
/// Counts the total number of set bits across all bytes in `data`, then
/// applies one of three formulas based on `bit_count % 3`:
///   0 → 222 − bit_count
///   1 → (bit_count / 2) + 111
///   2 → (bit_count / 3) + 177
fn calc_crc8(data: &[u8]) -> u8 {
  let bit_count: u32 = data.iter().map(|b| b.count_ones()).sum();
  let crc: u32 = match bit_count % 3 {
    0 => 222 - bit_count,
    1 => bit_count / 2 + 111,
    _ => bit_count / 3 + 177,
  };
  crc as u8
}

/// Build the full 18-byte BLE packet for a given command byte.
///
/// Packet layout:
///   [cmd, 0x64, 0x00, 0x00, 0x00, 0x00,
///    0x31, 0x32, 0x33, 0x34,          ← "1234" password
///    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, crc8]
fn make_cmd(command: u8) -> Vec<u8> {
  let mut data: Vec<u8> = vec![
    command, 0x64, 0x00, 0x00, 0x00, 0x00, 0x31, 0x32, 0x33, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00,
  ];
  let crc = calc_crc8(&data);
  data.push(crc);
  data
}

/// Validate a received BLE packet from the device by checking its length and CRC.
/// 
/// Packet layout:
///  [cmd, 0x64, 0x00, bitmask, 0x00, 0x00,
///   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
///   0x00, 0x00, 0x00, 0x00, crc8]
fn validate_response(data: &[u8]) -> bool {
  if data.len() != 18 {
    return false;
  }
  let crc = data[17];
  let expected_crc = calc_crc8(&data[0..17]);
  crc == expected_crc
  // Maybe return an object with multiple fields in the future.
  // { is_valid: bool, cmd: u8, on_off_held: bool, speed_up_held: bool, speed_down_held: bool, ... }
}

// Send a button press command followed by a release command, with error handling.
async fn send_button_press_cmd(
  device: &Arc<Hardware>,
  press_command: u8,
  release_command: u8,
) -> Result<(), ButtplugDeviceError> {
  let _result = device
    .write_value(&HardwareWriteCmd::new(
      &[FMACHINE_PROTOCOL_UUID],
      Endpoint::Tx,
      make_cmd(press_command),
      true,
    ))
    .await
    .map_err(|e| {
      ButtplugDeviceError::ProtocolSpecificError(
        "F-Machine".to_owned(),
        format!("Failed to send press command {press_command}: {e}"),
      )
    })?;
  // Maybe check response matches what we sent before sending release command?

  let _result = device
    .write_value(&HardwareWriteCmd::new(
      &[FMACHINE_PROTOCOL_UUID],
      Endpoint::Tx,
      make_cmd(release_command),
      true,
    ))
    .await
    .map_err(|e| {
      ButtplugDeviceError::ProtocolSpecificError(
        "F-Machine".to_owned(),
        format!("Failed to send release command {release_command}: {e}"),
      )
    })?;
  // Maybe check response matches what we sent before returning success?

  Ok(())
}

#[derive(Default)]
pub struct FMachineInitializer {}

#[async_trait]
impl ProtocolInitializer for FMachineInitializer {
  async fn initialize(
    &mut self,
    device: Arc<Hardware>,
    _: &ServerDeviceDefinition,
  ) -> Result<Arc<dyn ProtocolHandler>, ButtplugDeviceError> {
    warn!(
      "F-Machine device provides no state feedback. Speed and on/off state are tracked internally."
    );

    // Subscribe to the rx characteristic so any device notifications are captured.
    // The FMachine protocol documentation notes that the device *may* send notifications;
    // their meaning is currently unknown. A background task logs them for debugging.
    let mut event_receiver = device.event_stream();
    device
      .subscribe(&HardwareSubscribeCmd::new(
        FMACHINE_PROTOCOL_UUID,
        Endpoint::Rx,
      ))
      .await
      .map_err(|e| {
        ButtplugDeviceError::ProtocolSpecificError(
          "F-Machine".to_owned(),
          format!("Failed to subscribe to rx characteristic: {e}"),
        )
      })?;

    // For now just log any notifications received, in future we may want to use them
    // for button hold state detection.
    async_manager::spawn(async move {
      info!("F-Machine: BLE notification listener started");
      loop {
        select! {
          event = event_receiver.recv().fuse() => {
            match event {
              Ok(HardwareEvent::Notification(_, endpoint, data)) => {
                debug!("F-Machine notification on {:?}: {:02x?}", endpoint, data);
                if !validate_response(&data) {
                  warn!("F-Machine: received invalid notification data: {:02x?}", data);
                }
              }
              Ok(HardwareEvent::Disconnected(_)) => {
                info!("F-Machine: device disconnected, stopping notification listener");
                break;
              }
              Err(e) => {
                info!("F-Machine: notification listener error: {:?}", e);
                break;
              }
            }
          }
        }
      }
      info!("F-Machine: BLE notification listener exiting");
    });

    // Normalize the device's internally-remembered speed to 1 by sending 55 speed-down
    // press/release pairs at 60ms intervals. This mirrors the official app's remote-start
    // behaviour, ensuring our internal current_speed matches the device after connect.
    for _ in 0..FMACHINE_INIT_STEPS {
      send_button_press_cmd(&device, CMD_SPEED_DOWN, CMD_SPEED_RELEASE).await?;
      sleep(Duration::from_millis(FMACHINE_INIT_STEP_MS)).await;
    }

    Ok(Arc::new(FMachine::new(device)))
  }
}

// Protocol handler for F-Machine devices. The device provides no feedback on its state, so
// speed and on/off state are tracked internally. Commands are sent to adjust the device's
// state towards the current target whenever a new command is received. A background task
// continuously polls the target vs current state and sends appropriate commands to move
// the device towards the target.
//
// The F-Machine Tremblr BT-R and F-Machine Alpha, have secondary functions (air pump and
// oscillation distance) that are controlled by the same up/down command pattern as the
// primary function (oscillation speed).
//
// It is currently undecided how to handle the secondary functions as unlike the primary
// oscillation speed, they do not have discrete steps.
pub struct FMachine {
  is_running: Arc<AtomicBool>,
  current_speed: Arc<AtomicU8>,
  target_speed: Arc<AtomicU8>,
}

async fn update_handler(
  device: Arc<Hardware>,
  is_running: Arc<AtomicBool>,
  current_speed: Arc<AtomicU8>,
  target_speed: Arc<AtomicU8>,
) {
  info!("Entering F-Machine control loop");

  loop {
    let ir = is_running.load(Ordering::Relaxed);
    let tp = target_speed.load(Ordering::Relaxed);
    let cp = current_speed.load(Ordering::Relaxed);

    // Technically the on/off state is separate from the speed, but for simplicity we treat "off" as just speed 0.
    // If the device is on (ir ==  true), but target speed is 0, send an on/off press to turn it off.
    // Or if the device is off (ir == false), but target speed is not 0, send an on/off press to turn it on.
    if ir == (tp == 0) {
      trace!("F-Machine: on/off state {} → {}", ir, !ir);
      if send_button_press_cmd(&device, CMD_ON_OFF_PRESS, CMD_ON_OFF_RELEASE)
        .await
        .is_err()
      {
        info!("F-Machine on/off command error, most likely due to device disconnection.");
        break;
      };
      is_running.store(!ir, Ordering::Relaxed);
    }

    if tp != cp {
      let press_cmd = if tp > cp {
        CMD_SPEED_UP
      } else {
        CMD_SPEED_DOWN
      };
      trace!("F-Machine: primary speed {} → {}", cp, tp);
      if send_button_press_cmd(&device, press_cmd, CMD_SPEED_RELEASE)
        .await
        .is_err()
      {
        info!("F-Machine speed command error, most likely due to device disconnection.");
        break;
      };
      current_speed.store(if tp > cp { cp + 1 } else { cp - 1 }, Ordering::Relaxed);
    }

    sleep(Duration::from_millis(FMACHINE_COMMAND_TIMEOUT_MS)).await;
  }
  info!("F-Machine control loop exiting, most likely due to device disconnection.");
}

impl FMachine {
  fn new(device: Arc<Hardware>) -> Self {
    let is_running = Arc::new(AtomicBool::new(false));
    let current_speed = Arc::new(AtomicU8::new(0));
    let target_speed = Arc::new(AtomicU8::new(0));

    let is_running_clone = is_running.clone();
    let current_speed_clone = current_speed.clone();
    let target_speed_clone = target_speed.clone();

    async_manager::spawn(async move {
      update_handler(
        device,
        is_running_clone,
        current_speed_clone,
        target_speed_clone,
      )
      .await
    });
    Self {
      is_running,
      current_speed,
      target_speed,
    }
  }
}

// Currently only the primary oscillation speed function is implemented.
// No Secondary functions (suction level or thrust depth, depending on device model) are implemented.
// These secondary functions do not have discrete steps like the primary oscillation speed.
impl ProtocolHandler for FMachine {
  fn handle_output_oscillate_cmd(
    &self,
    feature_index: u32,
    _feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let speed: u8 = speed as u8;
    if feature_index == 0 {
      // Primary oscillation speed.
      self.target_speed.store(speed, Ordering::Relaxed);
    } else {
      warn!("Secondary function control for F-Machine is not currently implemented.");
    }
    Ok(vec![])
  }
}
