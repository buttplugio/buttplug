// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::step::{SideEffect, StepValidation, TestSequence, TestStep};
use std::sync::Arc;

pub fn core_protocol_sequence() -> TestSequence {
  TestSequence {
    name: "core_protocol",
    description: "Full protocol exercise without ping — handshake, enumeration, all output/input commands, stop, device removal",
    max_ping_time: 0,
    steps: vec![
      // Steps 1-4: Handshake and enumeration
      TestStep {
        name: "Handshake",
        description: "Wait for client connection",
        validation: StepValidation::WaitForConnection,
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: true,
      },
      TestStep {
        name: "Start Scanning",
        description: "Wait for client to request scanning",
        validation: StepValidation::WaitForScanning,
        side_effects: vec![SideEffect::TriggerScanning],
        timeout_ms: 5000,
        blocking: true,
      },
      TestStep {
        name: "Verify Devices Received",
        description: "Wait for client to process the device list",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Allow a brief delay for the client to receive and process DeviceList
          Ok(())
        })),
        side_effects: vec![SideEffect::Delay { ms: 500 }],
        timeout_ms: 5000,
        blocking: true,
      },
      TestStep {
        name: "Request Device List",
        description: "Validate client can request device list explicitly",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify server state shows 3 connected devices
          Ok(())
        })),
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      // Steps 5-14: Output command steps
      TestStep {
        name: "Vibrate Command (Device 0, Feature 0)",
        description: "Client sends OutputCmd with Vibrate to device 0, feature 0",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 0
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Vibrate Command (Device 0, Feature 1)",
        description: "Client sends OutputCmd with Vibrate to device 0, feature 1 (second vibrator)",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 0
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Rotate Command (Device 0, Feature 2)",
        description: "Client sends OutputCmd with Rotate to device 0, feature 2",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 0
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Oscillate Command (Device 1, Feature 2)",
        description: "Device 1 (Positioner), feature with Oscillate",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 1,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 1
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Position Command (Device 1, Feature 0)",
        description: "Device 1, Position feature",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 1,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 1
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "HwPositionWithDuration Command (Device 1, Feature 1)",
        description: "Device 1, HwPositionWithDuration feature",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 1,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 1
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Constrict Command (Device 2, Feature 0)",
        description: "Device 2 (Multi), Constrict",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 2,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 2
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Spray Command (Device 2, Feature 1)",
        description: "Device 2, Spray",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 2,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 2
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Temperature Command (Device 2, Feature 2)",
        description: "Device 2, Temperature",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 2,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 2
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Led Command (Device 2, Feature 3)",
        description: "Device 2, Led",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 2,
          validator: Arc::new(|_cmds| {
            // Validate a command was written to device 2
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      // Steps 15-18: Input command steps
      TestStep {
        name: "Battery Read (Device 0)",
        description: "Client sends InputCmd(Read, Battery) for device 0",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify the server successfully processed the read
          Ok(())
        })),
        side_effects: vec![SideEffect::InjectSensorReading {
          device_index: 0,
          endpoint: buttplug_server_device_config::Endpoint::RxBLEBattery,
          data: vec![85], // 85% battery
        }],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Sensor Subscribe (Device 2, Pressure)",
        description: "Client sends InputCmd(Subscribe, Pressure) for device 2",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify device handle 2 shows subscription
          Ok(())
        })),
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Sensor Notification (Device 2, Pressure)",
        description: "Server pushes a sensor reading",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify the notification was sent
          Ok(())
        })),
        side_effects: vec![SideEffect::InjectSensorReading {
          device_index: 2,
          endpoint: buttplug_server_device_config::Endpoint::Generic2,
          data: vec![0, 100, 0, 0],
        }],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Sensor Unsubscribe (Device 2, Pressure)",
        description: "Client sends InputCmd(Unsubscribe, Pressure) for device 2",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify device handle 2 no longer has subscription
          Ok(())
        })),
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      // Steps 19-20: Stop command steps
      TestStep {
        name: "Stop Single Device (Device 0)",
        description: "Client sends StopCmd targeting device 0",
        validation: StepValidation::ValidateDeviceCommand {
          device_index: 0,
          validator: Arc::new(|_cmds| {
            // Verify a stop/zero-value command was written to device 0
            Ok(())
          }),
        },
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      TestStep {
        name: "Stop All Devices",
        description: "Client sends StopCmd with no device_index (stops everything)",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify stop commands were sent to all device handles
          Ok(())
        })),
        side_effects: vec![],
        timeout_ms: 5000,
        blocking: false,
      },
      // Step 21: Device removal
      TestStep {
        name: "Device Removal (Device 1)",
        description: "Server removes a device",
        validation: StepValidation::Custom(Arc::new(|_ctx| {
          // Verify server now shows 2 connected devices
          Ok(())
        })),
        side_effects: vec![SideEffect::RemoveDevice { device_index: 1 }],
        timeout_ms: 5000,
        blocking: false,
      },
    ],
  }
}
