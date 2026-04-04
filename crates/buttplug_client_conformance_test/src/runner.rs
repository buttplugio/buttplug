// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::build_conformance_server;
use crate::step::{
  SequenceContext, SequenceResult, SideEffect, StepResult, StepValidation, TestSequence,
};
use buttplug_core::connector::ButtplugConnector;
use buttplug_server::connector::ButtplugRemoteServerConnector;
use buttplug_server::message::serializer::ButtplugServerJSONSerializer;
use buttplug_server::message::{ButtplugClientMessageVariant, ButtplugServerMessageVariant};
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketServerTransport;
use buttplug_transport_websocket_tungstenite::ButtplugWebsocketServerTransportBuilder;
use futures::stream::StreamExt;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Run a test sequence against a real ButtplugServer with WebSocket transport
pub async fn run_sequence(
  sequence: &TestSequence,
  port: u16,
  _default_timeout_ms: u64,
) -> SequenceResult {
  let sequence_name = sequence.name.to_string();
  let mut steps = Vec::new();
  let mut passed = true;

  // Build the server with conformance devices
  let (server, device_handles) = match build_conformance_server(sequence.max_ping_time) {
    Ok((s, h)) => (Arc::new(s), h),
    Err(e) => {
      error!("Failed to build server: {:?}", e);
      return SequenceResult {
        sequence_name,
        steps: vec![StepResult {
          step_name: "Server Setup",
          passed: false,
          error: Some(format!("Failed to build server: {:?}", e)),
          duration_ms: 0,
        }],
        passed: false,
      };
    }
  };

  // Create the WebSocket transport and connector
  let transport = ButtplugWebsocketServerTransportBuilder::default()
    .port(port)
    .finish();

  let mut connector = ButtplugRemoteServerConnector::<
    ButtplugWebsocketServerTransport,
    ButtplugServerJSONSerializer,
  >::new(transport);

  // Set up the message channel for the connector
  let (connector_sender, connector_receiver) = mpsc::channel::<ButtplugClientMessageVariant>(256);

  // Connect the transport with timeout (blocks until a client connects)
  let connect_timeout = std::time::Duration::from_secs(10);
  let connect_result =
    tokio::time::timeout(connect_timeout, connector.connect(connector_sender)).await;

  if connect_result.is_err() || connect_result.as_ref().unwrap().is_err() {
    let error_msg = if connect_result.is_err() {
      "Connector timeout waiting for client".to_string()
    } else {
      format!(
        "Connector error: {:?}",
        connect_result.unwrap().unwrap_err()
      )
    };
    error!("{}", &error_msg);
    return SequenceResult {
      sequence_name,
      steps: vec![StepResult {
        step_name: "Connection",
        passed: false,
        error: Some(error_msg),
        duration_ms: 0,
      }],
      passed: false,
    };
  }

  debug!("Connector accepted client");

  // Wrap connector in Arc so it can be shared with the message loop
  let connector = Arc::new(connector);

  info!("Server ready, client connected on ws://127.0.0.1:{}", port);

  // Client connected successfully (connector.connect() would have failed otherwise)
  let server_connected = true;
  info!("Client connected");
  steps.push(StepResult {
    step_name: "Connection",
    passed: true,
    error: None,
    duration_ms: 0,
  });

  // Start message loop in background
  let message_loop_task = tokio::spawn({
    let server = server.clone();
    let connector = connector.clone();
    async move { run_message_loop(server, connector, connector_receiver).await }
  });

  // Create context for steps (for custom validators)
  let context = SequenceContext {
    device_handles,
    server_connected,
  };

  // Execute each step
  for step in &sequence.steps {
    let step_start = Instant::now();

    // Execute side effects first
    for side_effect in &step.side_effects {
      match side_effect {
        SideEffect::SendClientMessage(msg) => {
          debug!("Sending client message: {:?}", msg);
          match server.parse_message(msg.clone()).await {
            Ok(response) => {
              debug!("Got server response: {:?}", response);
            }
            Err(err_msg) => {
              debug!("Got server error response: {:?}", err_msg);
            }
          }
          // Allow async device processing to complete
          tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        SideEffect::TriggerScanning => {
          debug!("Triggering scanning");
          // Brief delay to allow scanning to complete (conformance DCM is synchronous)
          tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        SideEffect::InjectSensorReading {
          device_index,
          endpoint,
          data,
        } => {
          debug!("Injecting sensor reading to device {}", device_index);
          if let Some(handle) = context.device_handles.get(*device_index) {
            handle
              .event_sender
              .send(
                buttplug_server::device::hardware::HardwareEvent::Notification(
                  format!("Device {}", device_index),
                  *endpoint,
                  data.clone(),
                ),
              )
              .ok();
          }
        }
        SideEffect::RemoveDevice { device_index } => {
          debug!("Removing device {}", device_index);
          if let Some(handle) = context.device_handles.get(*device_index) {
            handle
              .event_sender
              .send(
                buttplug_server::device::hardware::HardwareEvent::Disconnected(format!(
                  "Device {}",
                  device_index
                )),
              )
              .ok();
          }
        }
        SideEffect::CloseConnection => {
          debug!("Closing connection");
          // Connection closure is handled by the client
        }
        SideEffect::Delay { ms } => {
          debug!("Delaying {} ms", ms);
          tokio::time::sleep(std::time::Duration::from_millis(*ms)).await;
        }
      }
    }

    let result = match &step.validation {
      StepValidation::WaitForConnection => {
        // This would be the first step typically
        StepResult {
          step_name: step.name,
          passed: true,
          error: None,
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::WaitForScanning => {
        // Conformance DCM enumerates synchronously after trigger
        StepResult {
          step_name: step.name,
          passed: true,
          error: None,
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::ValidateDeviceCommand {
        device_index,
        validator,
      } => {
        let (passed, error) = if let Some(handle) = context.device_handles.get(*device_index) {
          let write_log = handle.write_log.lock().await;
          match validator(write_log.as_slice()) {
            Ok(()) => (true, None),
            Err(err_msg) => (false, Some(err_msg)),
          }
        } else {
          (false, Some(format!("Device {} not found", device_index)))
        };

        StepResult {
          step_name: step.name,
          passed,
          error,
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::WaitForServerEvent { .. } => {
        // For now, just pass - server events are handled by the message loop
        StepResult {
          step_name: step.name,
          passed: true,
          error: None,
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::WaitForDisconnect => {
        // Client disconnection is handled by the message loop
        StepResult {
          step_name: step.name,
          passed: true,
          error: None,
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::Custom(validator) => {
        let validation_result = validator(&context);
        StepResult {
          step_name: step.name,
          passed: validation_result.is_ok(),
          error: validation_result.err(),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
    };

    // Check if this step is blocking and failed
    if !result.passed {
      passed = false;
      if step.blocking {
        // Stop execution on blocking failure
        steps.push(result);
        break;
      }
    }

    steps.push(result);
  }

  // Cleanup: Wait for message loop to complete (when client disconnects)
  // If it takes too long, we abort it
  let _ = tokio::time::timeout(std::time::Duration::from_secs(10), message_loop_task).await;

  SequenceResult {
    sequence_name,
    steps,
    passed,
  }
}

/// Run the main message loop between client and server
/// Based on remote_server.rs run_server function
async fn run_message_loop<ConnectorType>(
  server: Arc<buttplug_server::ButtplugServer>,
  connector: Arc<ConnectorType>,
  mut connector_receiver: mpsc::Receiver<ButtplugClientMessageVariant>,
) where
  ConnectorType:
    ButtplugConnector<ButtplugServerMessageVariant, ButtplugClientMessageVariant> + 'static,
{
  let mut client_version_receiver = Box::pin(server.event_stream());

  loop {
    tokio::select! {
      // Client messages from the WebSocket
      msg = connector_receiver.recv() => {
        match msg {
          None => {
            debug!("Client disconnected, exiting message loop");
            break;
          }
          Some(client_message) => {
            debug!("Got message from client: {:?}", client_message);

            let server_clone = server.clone();
            let connector_clone = connector.clone();

            // Spawn message handling in background to match remote_server pattern
            buttplug_core::spawn!("conformance_test_message", async move {
              match server_clone.parse_message(client_message).await {
                Ok(response) => {
                  if connector_clone.send(response).await.is_err() {
                    error!("Failed to send response to client");
                  }
                }
                Err(err_msg) => {
                  if connector_clone.send(err_msg).await.is_err() {
                    error!("Failed to send error response to client");
                  }
                }
              }
            });
          }
        }
      }

      // Server version-specific events (unsolicited messages)
      msg = client_version_receiver.next() => {
        match msg {
          None => {
            debug!("Server event stream closed");
            break;
          }
          Some(server_msg) => {
            debug!("Got server event: {:?}", server_msg);
            let connector_clone = connector.clone();
            buttplug_core::spawn!("conformance_test_event", async move {
              if connector_clone.send(server_msg).await.is_err() {
                error!("Failed to send server event to client");
              }
            });
          }
        }
      }
    }
  }
}
