// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::device_manager::ConformanceDeviceHandle;
use buttplug_server::device::hardware::HardwareWriteCmd;
use buttplug_server_device_config::Endpoint;
use serde::Serialize;

/// A single test step that validates server/device state after a client action
#[derive(Clone)]
pub struct TestStep {
  pub name: &'static str,
  pub description: &'static str,
  pub validation: StepValidation,
  pub side_effects: Vec<SideEffect>,
  pub timeout_ms: u64,
  pub blocking: bool, // if true, failure aborts remaining steps
}

/// Describes what the runner checks after the client acts
#[derive(Clone)]
pub enum StepValidation {
  /// Wait for client to connect (observed via server event)
  WaitForConnection,
  /// Wait for scanning to start (observed via device manager state)
  WaitForScanning,
  /// Validate that specific device commands were captured in the write log
  ValidateDeviceCommand {
    device_index: usize,
    validator: std::sync::Arc<dyn Fn(&[HardwareWriteCmd]) -> Result<(), String> + Send + Sync>,
  },
  /// Wait for a server-initiated event to be received by the client
  WaitForServerEvent { description: String },
  /// Wait for client disconnection
  WaitForDisconnect,
  /// Custom validation function
  Custom(std::sync::Arc<dyn Fn(&SequenceContext) -> Result<(), String> + Send + Sync>),
}

/// Describes what the runner does before or during a step
#[derive(Clone)]
pub enum SideEffect {
  /// Send a client message directly to the server (bypassing WebSocket)
  SendClientMessage(buttplug_server::message::ButtplugClientMessageVariant),
  /// Trigger device scanning (adds pre-configured devices)
  TriggerScanning,
  /// Inject a sensor reading into a simulated device
  InjectSensorReading {
    device_index: usize,
    endpoint: Endpoint,
    data: Vec<u8>,
  },
  /// Remove a device (simulate disconnect)
  RemoveDevice { device_index: usize },
  /// Close the WebSocket connection from server side
  CloseConnection,
  /// Wait a fixed duration
  Delay { ms: u64 },
}

/// Result of a single test step
#[derive(Clone, Debug, Serialize)]
pub struct StepResult {
  pub step_name: &'static str,
  pub passed: bool,
  pub error: Option<String>,
  pub duration_ms: u64,
}

/// Result of a complete test sequence
#[derive(Clone, Debug, Serialize)]
pub struct SequenceResult {
  pub sequence_name: String,
  pub steps: Vec<StepResult>,
  pub passed: bool,
}

/// Provides access to runner state for custom validations
pub struct SequenceContext {
  pub device_handles: Vec<ConformanceDeviceHandle>,
  pub server_connected: bool,
}

/// Groups test steps with server configuration
pub struct TestSequence {
  pub name: &'static str,
  pub description: &'static str,
  pub max_ping_time: u32,
  pub steps: Vec<TestStep>,
}
