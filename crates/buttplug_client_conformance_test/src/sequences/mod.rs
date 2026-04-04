// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub mod core_protocol;

use crate::step::TestSequence;

/// Returns all available test sequences.
pub fn all_sequences() -> Vec<TestSequence> {
  vec![core_protocol::core_protocol_sequence()]
}
