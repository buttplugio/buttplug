// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::step::{SideEffect, StepValidation, TestSequence, TestStep};
use buttplug_core::message::{
  ButtplugClientMessageV4, ButtplugMessageSpecVersion, OutputCmdV4, OutputCommand, OutputValue,
  RequestServerInfoV4, StartScanningV0,
};
use buttplug_server::message::ButtplugClientMessageVariant;
use std::sync::Arc;

pub fn reconnection_sequence() -> TestSequence {
  TestSequence {
    name: "reconnection",
    description: "Validates client reconnects cleanly after server disconnect — fresh handshake, new device enumeration",
    max_ping_time: 0,
    steps: vec![
      // First connection: handshake and enumeration
      TestStep {
        name: "First Connection Handshake",
        description: "Standard handshake for first connection",
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
        name: "First Connection Scan",
        description: "Enumerate devices on first connection",
        validation: StepValidation::WaitForScanning,
        side_effects: vec![
          SideEffect::SendClientMessage(ButtplugClientMessageVariant::V4(
            ButtplugClientMessageV4::StartScanning(StartScanningV0::default()),
          )),
          SideEffect::TriggerScanning,
        ],
        timeout_ms: 5000,
        blocking: true,
      },
      // Server-initiated disconnect
      TestStep {
        name: "Server Closes Connection",
        description: "Server closes the WebSocket connection",
        validation: StepValidation::WaitForDisconnect,
        side_effects: vec![SideEffect::CloseConnection],
        timeout_ms: 5000,
        blocking: true,
      },
      // Rebuild server for second connection
      TestStep {
        name: "Rebuild Server for Reconnection",
        description: "Tear down and rebuild server fresh on same port",
        validation: StepValidation::WaitForConnection,
        side_effects: vec![SideEffect::RebuildServer],
        timeout_ms: 10000,
        blocking: true,
      },
      // Second connection: fresh handshake and enumeration
      TestStep {
        name: "Reconnection Handshake",
        description: "Handshake after reconnection",
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
        name: "Reconnection Scan",
        description: "Fresh device enumeration after reconnect",
        validation: StepValidation::WaitForScanning,
        side_effects: vec![
          SideEffect::SendClientMessage(ButtplugClientMessageVariant::V4(
            ButtplugClientMessageV4::StartScanning(StartScanningV0::default()),
          )),
          SideEffect::TriggerScanning,
        ],
        timeout_ms: 5000,
        blocking: true,
      },
      TestStep {
        name: "Reconnection Device Command",
        description: "Verify device commands work on reconnected session",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|write_log| {
            // Verify that at least one message was sent to device 0
            if write_log.is_empty() {
              Err("No commands sent to device 0".to_string())
            } else {
              Ok(())
            }
          }),
        },
        side_effects: vec![SideEffect::SendClientMessage(
          ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::OutputCmd(OutputCmdV4::new(
            0,
            0,
            OutputCommand::Vibrate(OutputValue::new(50)),
          ))),
        )],
        timeout_ms: 5000,
        blocking: false,
      },
    ],
  }
}
