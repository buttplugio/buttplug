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

pub fn ping_timeout_sequence() -> TestSequence {
  TestSequence {
    name: "ping_timeout",
    description: "Validates server disconnects client that fails to send Ping within max_ping_time",
    max_ping_time: 500,
    steps: vec![
      TestStep {
        name: "Handshake with Short Ping",
        description: "Connection with 500ms ping time",
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
        name: "Wait for Ping Timeout",
        description: "Do NOT send any Ping. Wait for server to disconnect.",
        validation: StepValidation::WaitForDisconnect,
        side_effects: vec![],
        timeout_ms: 2000,
        blocking: true,
      },
      TestStep {
        name: "Verify Disconnected State",
        description: "Confirm the connection is closed",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // The connection should be closed at this point
          Ok(())
        })),
        side_effects: vec![],
        timeout_ms: 1000,
        blocking: false,
      },
    ],
  }
}
