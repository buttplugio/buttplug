// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::step::{SideEffect, StepValidation, TestSequence, TestStep};
use buttplug_core::message::{
  ButtplugClientMessageV4, OutputCmdV4, OutputCommand, OutputValue, RequestServerInfoV4,
  ButtplugMessageSpecVersion, StartScanningV0,
};
use buttplug_server::message::ButtplugClientMessageVariant;
use buttplug_server_device_config::Endpoint;
use std::sync::Arc;

pub fn error_handling_sequence() -> TestSequence {
  TestSequence {
    name: "error_handling",
    description: "Validates client handles error responses and continues operating",
    max_ping_time: 0,
    steps: vec![
      TestStep {
        name: "Handshake",
        description: "Standard connection",
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
        name: "Scan and Enumerate",
        description: "Set up devices for subsequent tests",
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
        name: "Invalid Device Index",
        description: "Client sends OutputCmd to non-existent device index (e.g., 99)",
        validation: StepValidation::Custom(Arc::new(|ctx| {
          // Verify the client remains connected after this error
          if ctx.server_connected {
            Ok(())
          } else {
            Err("Server should still be connected after error".to_string())
          }
        })),
        side_effects: vec![SideEffect::SendClientMessage(
          ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::OutputCmd(OutputCmdV4::new(
            99,
            0,
            OutputCommand::Vibrate(OutputValue::new(50)),
          ))),
        )],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Valid Command After Error",
        description: "Client sends a valid OutputCmd to device 0",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|cmds| {
            if cmds.is_empty() {
              return Err("No commands written to device 0".to_string());
            }
            let last_cmd = &cmds[cmds.len() - 1];
            if last_cmd.endpoint() != Endpoint::Tx {
              return Err(format!(
                "Expected Endpoint::Tx, got {:?}",
                last_cmd.endpoint()
              ));
            }
            Ok(())
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
      TestStep {
        name: "Invalid Feature Index",
        description: "Client sends OutputCmd to device 0 with invalid feature_index (e.g., 99)",
        validation: StepValidation::Custom(Arc::new(|ctx| {
          // Verify server returns error and client stays connected
          if ctx.server_connected {
            Ok(())
          } else {
            Err("Server should still be connected after feature error".to_string())
          }
        })),
        side_effects: vec![SideEffect::SendClientMessage(
          ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::OutputCmd(OutputCmdV4::new(
            0,
            99,
            OutputCommand::Vibrate(OutputValue::new(50)),
          ))),
        )],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Valid Command After Second Error",
        description: "Another valid command to prove continued operation",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|cmds| {
            if cmds.is_empty() {
              return Err("No commands written to device 0".to_string());
            }
            let last_cmd = &cmds[cmds.len() - 1];
            if last_cmd.endpoint() != Endpoint::Tx {
              return Err(format!(
                "Expected Endpoint::Tx, got {:?}",
                last_cmd.endpoint()
              ));
            }
            Ok(())
          }),
        },
        side_effects: vec![SideEffect::SendClientMessage(
          ButtplugClientMessageVariant::V4(ButtplugClientMessageV4::OutputCmd(OutputCmdV4::new(
            0,
            0,
            OutputCommand::Rotate(OutputValue::new(75)),
          ))),
        )],
        timeout_ms: 5000,
        blocking: false,
      },
    ],
  }
}
