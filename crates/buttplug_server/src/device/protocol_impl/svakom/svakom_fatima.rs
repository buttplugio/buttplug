// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

// Svakom Fatima Pro (BLE advertised name "SL278B").
//
// Protocol derived from a BLE packet capture of the official app:
//   - Device advertises as SL278B; writes go to FFE1. Writes are Write Without Response.
//   - General command form: 55 <func> 00 00 <mode> <intensity>
//       Vibration  func=03: mode 1-10, intensity 0-10  (steady = mode 01 + intensity)
//       Suction    func=09: mode 1-5,  intensity 0-10
//       Thrust     func=08: mode 1-7,  trailing byte fixed 0xff (discrete patterns only)
//       Heat       func=05: on  55 05 01 37 02 00 00 / off 55 05 00 00 02 00 00
//     Per-function off = 55 <func> 00 00 00 00.
//   - No init handshake / notification subscribe is required. The official app sends a
//     handshake on connect (55 00 / 55 04 .. aa -> 55 80 .. status/battery read on FFE2),
//     but hardware testing (cold boot, raw writes to FFE1) showed the device drives with
//     just connect + write, so the protocol has no initializer.
//
// Buttplug mapping: vibration -> Vibrate[0,10], suction -> Constrict[0,10],
//                   thrust -> Oscillate[0,7], heat -> Temperature[0,1].
// See the device-config entry (svakom-fatima.yml) for the feature definitions.

use crate::device::{
  hardware::{HardwareCommand, HardwareWriteCmd},
  protocol::{ProtocolHandler, ProtocolKeepaliveStrategy, generic_protocol_setup},
};
use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::Endpoint;
use uuid::Uuid;

generic_protocol_setup!(SvakomFatima, "svakom-fatima");

#[derive(Default)]
pub struct SvakomFatima {}

impl SvakomFatima {
  // Vibration/suction share a form: steady mode (01) + intensity; intensity 0 = off.
  fn steady(func: u8, speed: u32) -> Vec<u8> {
    if speed == 0 {
      vec![0x55, func, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![0x55, func, 0x00, 0x00, 0x01, speed as u8]
    }
  }
}

impl ProtocolHandler for SvakomFatima {
  fn keepalive_strategy(&self) -> ProtocolKeepaliveStrategy {
    ProtocolKeepaliveStrategy::HardwareRequiredRepeatLastPacketStrategy
  }

  // Vibration: 55 03 00 00 01 <0-10>
  fn handle_output_vibrate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, Self::steady(0x03, speed), false).into(),
    ])
  }

  // Suction: 55 09 00 00 01 <0-10>
  fn handle_output_constrict_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    Ok(vec![
      HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, Self::steady(0x09, speed), false).into(),
    ])
  }

  // Thrust: 55 08 00 00 <pattern 1-7> ff ; off = 55 08 00 00 00 00
  // The device exposes discrete firmware patterns, not a continuous speed, so the
  // Oscillate value selects a pattern number (see svakom-fatima.yml / the PR notes).
  fn handle_output_oscillate_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: u32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let pkt = if speed == 0 {
      vec![0x55, 0x08, 0x00, 0x00, 0x00, 0x00]
    } else {
      vec![0x55, 0x08, 0x00, 0x00, speed as u8, 0xff]
    };
    Ok(vec![HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, pkt, false).into()])
  }

  // Heat: on 55 05 01 37 02 00 00 ; off 55 05 00 00 02 00 00 (on/off only).
  fn handle_output_temperature_cmd(
    &self,
    _feature_index: u32,
    feature_id: Uuid,
    speed: i32,
  ) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
    let pkt = if speed == 0 {
      vec![0x55, 0x05, 0x00, 0x00, 0x02, 0x00, 0x00]
    } else {
      vec![0x55, 0x05, 0x01, 0x37, 0x02, 0x00, 0x00]
    };
    Ok(vec![HardwareWriteCmd::new(&[feature_id], Endpoint::Tx, pkt, false).into()])
  }
}
