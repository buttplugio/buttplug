// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::step::{StepValidation, TestSequence, TestStep};
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_handshake_sequence() {
  // Create a minimal handshake-only sequence
  let sequence = TestSequence {
    name: "Handshake",
    description: "Test basic server handshake",
    max_ping_time: 0,
    steps: vec![TestStep {
      name: "WaitConnection",
      description: "Wait for client connection",
      validation: StepValidation::WaitForConnection,
      side_effects: vec![],
      timeout_ms: 2000,
      blocking: true,
    }],
  };

  // Use a random high port to avoid conflicts
  let port = 20000
    + (std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs()
      % 10000) as u16;

  // Spawn the runner in a background task
  let runner_task = tokio::spawn(async move { run_sequence(&sequence, port, 2000).await });

  // Wait for server to start listening
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

  // Connect as a client
  let ws_url = format!("ws://127.0.0.1:{}", port);
  let connect_result = async {
    match connect_async(&ws_url).await {
      Ok((mut ws, _)) => {
        // Send RequestServerInfo message
        let handshake_msg =
          r#"[{"RequestServerInfo":{"Id":1,"ClientName":"Test","ProtocolVersionMajor":4}}]"#;
        if ws.send(Message::Text(handshake_msg.into())).await.is_ok() {
          // Receive ServerInfo response (or wait for something)
          let _response = ws.next().await;
        }
      }
      Err(_e) => {
        // If we can't connect, that's ok - runner handles timeout
      }
    }
  };

  let _ = tokio::time::timeout(std::time::Duration::from_secs(3), connect_result).await;

  // Wait for the runner to complete (with timeout)
  let runner_result = tokio::time::timeout(std::time::Duration::from_secs(10), runner_task)
    .await
    .expect("Runner task timed out")
    .expect("Runner task panicked");

  // The runner should complete without panicking and the test should pass
  // This is a baseline integration test that the runner doesn't crash
  assert!(
    !runner_result.sequence_name.is_empty(),
    "Result should have sequence name"
  );
  assert!(runner_result.passed, "Handshake sequence should pass");
}
