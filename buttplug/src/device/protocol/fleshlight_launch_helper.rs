// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2022 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[allow(dead_code)]
pub fn calculate_distance(duration: u32, mut speed: f64) -> f64 {
  if speed <= 0f64 {
    return 0f64;
  }

  if speed > 1f64 {
    speed = 1f64;
  }

  let mil = (speed / 250f64).powf(-0.95);
  let diff = mil - (duration as f64);
  if diff.abs() < 0.001 {
    0f64
  } else {
    ((90f64 - (diff / mil * 90f64)) / 100f64)
      .min(1f64)
      .max(0f64)
  }
}

pub fn calculate_speed(mut distance: f64, duration: u32) -> f64 {
  if distance < 0f64 {
    return 0f64;
  }

  if distance > 1f64 {
    distance = 1f64;
  }

  let scalar = ((duration as f64 * 90f64) / (distance * 100f64)).powf(-1.05);

  250f64 * scalar
}

pub fn calculate_duration(mut distance: f64, mut speed: f64) -> u32 {
  if distance <= 0f64 || speed <= 0f64 {
    return 0;
  }

  if distance > 1f64 {
    distance = 1f64;
  }

  if speed > 1f64 {
    speed = 1f64;
  }

  let mil = (speed / 250f64).powf(-0.95);
  (mil / (90f64 / (distance * 100f64))) as u32
}
