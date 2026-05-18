// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Output observation type for output command tracking and observability

#[derive(Clone, Debug)]
pub struct OutputObservation {
  pub device_index: u32,
  pub feature_index: u32,
  pub output_type: String,
  pub value: f64,
}
