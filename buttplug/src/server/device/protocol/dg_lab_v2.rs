// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2023 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::{core::errors::ButtplugDeviceError, server::device::protocol::{generic_protocol_setup, ProtocolHandler}};
use crate::core::errors::ButtplugDeviceError::ProtocolSpecificError;
use crate::core::message::{ActuatorType, Endpoint};
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

fn xyz_to_bytes(x: u32, y: u32, z: u32) -> Vec<u8> {
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
pub struct DGLabV2 {}

impl ProtocolHandler for DGLabV2 {
    fn needs_full_command_set(&self) -> bool {
        true
    }

    fn handle_scalar_cmd(&self, commands: &[Option<(ActuatorType, u32)>]) -> Result<Vec<HardwareCommand>, ButtplugDeviceError> {
        // Power A (S)
        let mut power_a_scalar: u32 = 0;
        // Power B (S)
        let mut power_b_scalar: u32 = 0;
        // Frequency A (X, Y)
        let mut xy_a_scalar: (u32, u32) = (0, 0);
        // Frequency B (X, Y)
        let mut xy_b_scalar: (u32, u32) = (0, 0);
        // Pulse width A (Z)
        let mut pulse_width_a_scalar: u32 = 0;
        // Pulse width B (Z)
        let mut pulse_width_b_scalar: u32 = 0;
        for (index, command) in commands.iter().enumerate().filter(|(_, x)| x.is_some()) {
            let (actuator, mut scalar) = command.as_ref().expect("Already verified existence");
            match *actuator {
                // Set power (S)
                ActuatorType::Vibrate => {
                    if scalar > MAXIMUM_POWER {
                        return Err(
                            ProtocolSpecificError(
                                "dg-lab-v2".to_owned(),
                                format!("Power scalar {} not in [0, {}]", scalar, MAXIMUM_POWER),
                            )
                        );
                    }
                    match index {
                        // Channel A
                        0 => { power_a_scalar = scalar; }
                        // Channel B
                        1 => { power_b_scalar = scalar; }
                        _ => {
                            return Err(
                                ProtocolSpecificError(
                                    "dg-lab-v2".to_owned(),
                                    format!("Vibrate command index {} is invalid", index),
                                )
                            );
                        }
                    }
                }
                // Set frequency (X, Y)
                ActuatorType::Oscillate => {
                    if scalar == 0 {
                        scalar = MINIMUM_FREQUENCY;
                    } else if scalar < MINIMUM_FREQUENCY || scalar > MAXIMUM_FREQUENCY {
                        return Err(
                            ProtocolSpecificError(
                                "dg-lab-v2".to_owned(),
                                format!("Frequency scalar {} not in [{}, {}]", scalar, MINIMUM_FREQUENCY, MAXIMUM_FREQUENCY),
                            )
                        );
                    }
                    match index {
                        // Channel A
                        2 => { xy_a_scalar = frequency_to_xy(scalar); }
                        // Channel B
                        3 => { xy_b_scalar = frequency_to_xy(scalar); }
                        _ => {
                            return Err(
                                ProtocolSpecificError(
                                    "dg-lab-v2".to_owned(),
                                    format!("Oscillate command index {} is invalid", index),
                                )
                            );
                        }
                    }
                }
                // Set pulse width (Z)
                ActuatorType::Inflate => {
                    if scalar > MAXIMUM_PULSE_WIDTH {
                        return Err(
                            ProtocolSpecificError(
                                "dg-lab-v2".to_owned(),
                                format!("Pulse width scalar {} not in [0, {}]", scalar, MAXIMUM_PULSE_WIDTH),
                            )
                        );
                    }
                    match index {
                        // Channel A
                        4 => { pulse_width_a_scalar = scalar; }
                        // Channel B
                        5 => { pulse_width_b_scalar = scalar; }
                        _ => {
                            return Err(
                                ProtocolSpecificError(
                                    "dg-lab-v2".to_owned(),
                                    format!("Inflate command index {} is invalid", index),
                                )
                            );
                        }
                    }
                }
                _ => {
                    return Err(ButtplugDeviceError::UnhandledCommand(
                        "Unknown actuator types are not controllable.".to_owned(),
                    ));
                }
            }
        }
        Ok(
            vec![
                HardwareWriteCmd::new(
                    Endpoint::Tx,
                    ab_power_to_byte(power_a_scalar, power_b_scalar),
                    false,
                ).into(),
                HardwareWriteCmd::new(
                    Endpoint::Generic0,
                    xyz_to_bytes(xy_a_scalar.0, xy_a_scalar.1, pulse_width_a_scalar),
                    false,
                ).into(),
                HardwareWriteCmd::new(
                    Endpoint::Generic1,
                    xyz_to_bytes(xy_b_scalar.0, xy_b_scalar.1, pulse_width_b_scalar),
                    false,
                ).into(),
            ]
        )
    }
}