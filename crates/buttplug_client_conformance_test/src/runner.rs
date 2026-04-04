// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use crate::server::build_conformance_server;
use crate::step::{SequenceContext, SequenceResult, StepResult, StepValidation, TestSequence};
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
  default_timeout_ms: u64,
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

  // Connect the transport (blocks until a client connects)
  if let Err(e) = connector.connect(connector_sender).await {
    error!("Connector error: {:?}", e);
    return SequenceResult {
      sequence_name,
      steps: vec![StepResult {
        step_name: "Connection",
        passed: false,
        error: Some(format!("Connector failed: {:?}", e)),
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

  // Create context for steps (for future custom validators)
  let _context = SequenceContext {
    device_handles,
    server_connected,
  };

  // Execute each step
  for step in &sequence.steps {
    let step_start = Instant::now();
    let _timeout_ms = if step.timeout_ms > 0 {
      step.timeout_ms
    } else {
      default_timeout_ms
    };

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
        // Placeholder for now
        StepResult {
          step_name: step.name,
          passed: false,
          error: Some("WaitForScanning not yet implemented".to_string()),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::ValidateDeviceCommand { .. } => {
        // Placeholder for now
        StepResult {
          step_name: step.name,
          passed: false,
          error: Some("ValidateDeviceCommand not yet implemented".to_string()),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::WaitForServerEvent { .. } => {
        // Placeholder for now
        StepResult {
          step_name: step.name,
          passed: false,
          error: Some("WaitForServerEvent not yet implemented".to_string()),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::WaitForDisconnect => {
        // Placeholder for now
        StepResult {
          step_name: step.name,
          passed: false,
          error: Some("WaitForDisconnect not yet implemented".to_string()),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
      StepValidation::Custom(_validator) => {
        // Placeholder - would call validator with context
        StepResult {
          step_name: step.name,
          passed: false,
          error: Some("Custom validation not yet implemented".to_string()),
          duration_ms: step_start.elapsed().as_millis() as u64,
        }
      }
    };

    if !result.passed && step.blocking {
      passed = false;
      steps.push(result);
      break; // Stop on blocking failure
    }

    if !result.passed {
      passed = false;
    }

    steps.push(result);
  }

  // Cleanup
  let _ = message_loop_task.abort();

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
