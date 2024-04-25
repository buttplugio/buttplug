// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use std::sync::RwLock;

use crate::{core::errors::ButtplugDeviceError, server::device::protocol::{generic_protocol_setup, ProtocolHandler}};
use crate::core::errors::ButtplugDeviceError::ProtocolSpecificError;
use crate::core::message::Endpoint;
use crate::server::device::hardware::{HardwareCommand, HardwareWriteCmd};

static MINIMUM_FREQUENCY: u32 = 10;
static MAXIMUM_FREQUENCY: u32 = 1000;
static MAXIMUM_POWER: u32 = 2047;
static MAXIMUM_PULSE_WIDTH: u32 = 31;
static MAXIMUM_X: f32 = 31f32;
static MAXIMUM_Y: f32 = 1023f32;


fn ab_power_to_byte(a: u32, b: u32) -> Vec<u8> {
    let data = 0 | ((b & 0x7FF) << 11) | (a & 0x7FF);
    return vec![
        (data & 0xFF) as u8,
        ((data >> 8) & 0xFF) as u8,
        ((data >> 16) & 0xFF) as u8,
    ];
}

fn xyz_to_byte(x: u32, y: u32, z: u32) -> Vec<u8> {
    let data = 0 | ((z & 0x1F) << 15) | ((y & 0x3FF) << 5) | (x & 0x1F);
    return vec![
        (data & 0xFF) as u8,
        ((data >> 8) & 0xFF) as u8,
        ((data >> 16) & 0xFF) as u8,
    ];
}

fn frequency_to_xy(frequency: u32) -> (u32, u32) {
    let mut x = (frequency as f32 / 1000f32).sqrt() * 15f32;
    let mut y = frequency as f32 - x;
    if x > MAXIMUM_X { x = MAXIMUM_X }
    if y > MAXIMUM_Y { y = MAXIMUM_Y }
    return (x.round() as u32, y.round() as u32);
}

generic_protocol_setup!(DGLabV2, "dg-lab-v2");

#[derive(Default)]
pub struct DGLabV2 {
    /// Power A (S)
    power_a_scalar: RwLock<u32>,
    /// Power B (S)
    power_b_scalar: RwLock<u32>,
    /// Frequency A (X, Y)
    xy_a_scalar: RwLock<(u32, u32)>,
    /// Frequency B (X, Y)
    xy_b_scalar: RwLock<(u32, u32)>,
    /// Pulse width A (Z)
    pulse_width_a_scalar: RwLock<u32>,
    /// Pulse width B (Z)
    pulse_width_b_scalar: RwLock<u32>,
}

impl DGLabV2 {}

impl ProtocolHandler for DGLabV2 {
    fn needs_full_command_set(&self) -> bool {
        true
    }

    /// Set power (S)
    fn handle_scalar_vibrate_cmd(&self, index: u32, scalar: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        if scalar > MAXIMUM_POWER {
            return Err(
                ProtocolSpecificError(
                    "dg-lab-v2".to_owned(),
                    format!("Power scalar {} not in [0, {}]", scalar, MAXIMUM_POWER),
                )
            );
        }
        return match index {
            // Channel A
            0 => {
                let mut power_a_scalar_writer = self.power_a_scalar.write().expect("");
                *power_a_scalar_writer = scalar;
                Ok(vec![])
            }
            // Channel B
            1 => {
                let power_a_scalar = self.power_a_scalar.read().unwrap().clone();
                let mut power_b_scalar_writer = self.power_b_scalar.write().expect("");
                *power_b_scalar_writer = scalar;
                Ok(
                    vec![
                        HardwareWriteCmd::new(
                            Endpoint::Tx,
                            ab_power_to_byte(power_a_scalar, scalar),
                            false,
                        ).into()
                    ]
                )
            }
            _ => {
                Err(
                    ProtocolSpecificError(
                        "dg-lab-v2".to_owned(),
                        format!("Vibrate command index {} is invalid", index),
                    )
                )
            }
        };
    }

    /// Set frequency (X, Y)
    fn handle_scalar_oscillate_cmd(&self, index: u32, scalar: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        if scalar == 0 {
            return self.handle_scalar_oscillate_cmd(index, 10);
        }
        if scalar < MINIMUM_FREQUENCY || scalar > MAXIMUM_FREQUENCY {
            return Err(
                ProtocolSpecificError(
                    "dg-lab-v2".to_owned(),
                    format!("Frequency scalar {} not in [{}, {}]", scalar, MINIMUM_FREQUENCY, MAXIMUM_FREQUENCY),
                )
            );
        }
        return match index {
            // Channel A
            2 => {
                let pulse_width_scalar = self.pulse_width_a_scalar.read().unwrap().clone();
                let mut xy_scalar_writer = self.xy_a_scalar.write().expect("");
                let (x_scalar, y_scalar) = frequency_to_xy(scalar);
                *xy_scalar_writer = (x_scalar, y_scalar);
                Ok(
                    vec![
                        HardwareWriteCmd::new(
                            Endpoint::Generic0,
                            xyz_to_byte(x_scalar, y_scalar, pulse_width_scalar),
                            false,
                        ).into()
                    ]
                )
            }
            // Channel B
            3 => {
                let pulse_width_scalar = self.pulse_width_b_scalar.read().unwrap().clone();
                let mut xy_scalar_writer = self.xy_b_scalar.write().expect("");
                let (x_scalar, y_scalar) = frequency_to_xy(scalar);
                *xy_scalar_writer = (x_scalar, y_scalar);
                Ok(
                    vec![
                        HardwareWriteCmd::new(
                            Endpoint::Generic1,
                            xyz_to_byte(x_scalar, y_scalar, pulse_width_scalar),
                            false,
                        ).into()
                    ]
                )
            }
            _ => {
                Err(
                    ProtocolSpecificError(
                        "dg-lab-v2".to_owned(),
                        format!("Oscillate command index {} is invalid", index),
                    )
                )
            }
        };
    }

    /// Set pulse width (Z)
    fn handle_scalar_inflate_cmd(&self, index: u32, scalar: u32) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        if scalar > MAXIMUM_PULSE_WIDTH {
            return Err(
                ProtocolSpecificError(
                    "dg-lab-v2".to_owned(),
                    format!("Pulse width scalar {} not in [0, {}]", scalar, MAXIMUM_PULSE_WIDTH),
                )
            );
        }
        return match index {
            // Channel A
            4 => {
                let (x_scalar, y_scalar) = self.xy_a_scalar.read().unwrap().clone();
                let mut pulse_width_writer = self.pulse_width_a_scalar.write().expect("");
                *pulse_width_writer = scalar;
                Ok(
                    vec![
                        HardwareWriteCmd::new(
                            Endpoint::Generic0,
                            xyz_to_byte(x_scalar, y_scalar, scalar),
                            false,
                        ).into()
                    ]
                )
            }
            // Channel B
            5 => {
                let (x_scalar, y_scalar) = self.xy_b_scalar.read().unwrap().clone();
                let mut pulse_width_writer = self.pulse_width_b_scalar.write().expect("");
                *pulse_width_writer = scalar;
                Ok(
                    vec![
                        HardwareWriteCmd::new(
                            Endpoint::Generic1,
                            xyz_to_byte(x_scalar, y_scalar, scalar),
                            false,
                        ).into()
                    ]
                )
            }
            _ => {
                Err(
                    ProtocolSpecificError(
                        "dg-lab-v2".to_owned(),
                        format!("Inflate command index {} is invalid", index),
                    )
                )
            }
        };
    }
}