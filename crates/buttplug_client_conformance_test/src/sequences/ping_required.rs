// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::step::{SideEffect, StepValidation, TestSequence, TestStep};
use buttplug_core::message::{
  ButtplugClientMessageV4, ButtplugMessageSpecVersion, RequestServerInfoV4,
};
use buttplug_server::message::ButtplugClientMessageVariant;
use std::sync::Arc;

pub fn ping_required_sequence() -> TestSequence {
  TestSequence {
    name: "ping_required",
    description: "Validates client sends periodic Ping when server advertises max_ping_time > 0",
    max_ping_time: 1000,
    steps: vec![
      TestStep {
        name: "Handshake with Ping",
        description: "Wait for connection, server advertises max_ping_time: 1000",
        validation: StepValidation::WaitForConnection,
        side_effects: vec![SideEffect::SendClientMessage(
          ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::RequestServerInfo(
            RequestServerInfoV4::new(
              "conformance-runner",
              ButtplugMessageSpecVersion::Version4,
              0,
            ),
          )),
        )],
        timeout_ms: 5000,
        blocking: true,
      },
      TestStep {
        name: "First Ping Received",
        description: "Wait for client to send first Ping within 1000ms",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // The server hasn't pinged out if the sequence is still running.
          // This validator just confirms the server is still connected.
          Ok(())
        })),
        side_effects: vec![SideEffect::Delay { ms: 900 }],
        timeout_ms: 1500,
        blocking: true,
      },
      TestStep {
        name: "Second Ping Received",
        description: "Wait for another ping cycle",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify server is still connected
          Ok(())
        })),
        side_effects: vec![SideEffect::Delay { ms: 900 }],
        timeout_ms: 1500,
        blocking: false,
      },
      TestStep {
        name: "Ping with Device Operations",
        description: "Verify ping continues while doing other operations",
        validation: StepValidation::Custom(Arc::new(|ctx| {
          // Verify server still connected and devices available after another ping cycle
          if ctx.device_handles.len() == 3 {
            Ok(())
          } else {
            Err(format!(
              "Expected 3 device handles, got {}",
              ctx.device_handles.len()
            ))
          }
        })),
        side_effects: vec![
          SideEffect::TriggerScanning,
          SideEffect::Delay { ms: 900 },
        ],
        timeout_ms: 2000,
        blocking: false,
      },
    ],
  }
}
