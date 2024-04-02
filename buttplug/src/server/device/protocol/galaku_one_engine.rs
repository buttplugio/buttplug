// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{
    core::{errors::ButtplugDeviceError, message::Endpoint},
    server::device::{
        hardware::{HardwareCommand, HardwareWriteCmd},
        protocol::{generic_protocol_setup, ProtocolHandler},
    },
};

static KEY_TAB: [[u32; 12]; 4] = [
    [0, 24, 152, 247, 165, 61, 13, 41, 37, 80, 68, 70],
    [0, 69, 110, 106, 111, 120, 32, 83, 45, 49, 46, 55],
    [0, 101, 120, 32, 84, 111, 121, 115, 10, 142, 157, 163],
    [0, 197, 214, 231, 248, 10, 50, 32, 111, 98, 13, 10]
];

fn get_tab_key(r: usize, t: usize) -> u32 {
    let e = 3 & r;
    return KEY_TAB[e][t];
}

fn encrypt(data: Vec<u32>) -> Vec<u32> {
    let mut new_data = vec![data[0]];
    for i in 1..data.len() {
        let a = get_tab_key(new_data[i - 1] as usize, i);
        let u = (a ^ data[0] ^ data[i]) + a;
        new_data.push(u);
    }
    return new_data;
}

fn send_bytes(data: Vec<u32>) -> Vec<u8> {
    let mut new_data = vec![35];
    new_data.extend(data);
    new_data.push(new_data.iter().sum());
    let mut uint8_array: Vec<u8> = Vec::new();
    for value in encrypt(new_data) {
        uint8_array.push(value as u8);
    }
    return uint8_array;
}

generic_protocol_setup!(GalakuOneEngine, "galaku-one-engine");

#[derive(Default)]
pub struct GalakuOneEngine {}

impl ProtocolHandler for GalakuOneEngine {
    fn keepalive_strategy(&self) -> super::ProtocolKeepaliveStrategy {
        super::ProtocolKeepaliveStrategy::RepeatLastPacketStrategy
    }

    fn handle_scalar_vibrate_cmd(&self, _index: u32, scalar: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        let data: Vec<u32> = vec![90, 0, 0, 1, 49, scalar, 0, 0, 0, 0];
        Ok(vec![HardwareWriteCmd::new(Endpoint::Tx, send_bytes(data), true).into()])
    }
}
