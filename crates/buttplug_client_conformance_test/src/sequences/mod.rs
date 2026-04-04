// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

pub mod core_protocol;
pub mod error_handling;
pub mod ping_required;
pub mod ping_timeout;
pub mod reconnection;

use crate::step::TestSequence;

/// Returns all available test sequences.
pub fn all_sequences() -> Vec<TestSequence> {
  vec![
    core_protocol::core_protocol_sequence(),
    ping_required::ping_required_sequence(),
    error_handling::error_handling_sequence(),
    ping_timeout::ping_timeout_sequence(),
    reconnection::reconnection_sequence(),
  ]
}
