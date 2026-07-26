// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Listener integration tests: ephemeral-port binding, local_addr, loopback-
//! only binding, and graceful shutdown via the cancellation token. These verify
//! the public API surface the engine will use to bind before serving.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, TcpStream};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::TaskWebServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_port_zero_returns_ephemeral_loopback_addr() {
  let server = TaskWebServer::bind(0).await.expect("bind port 0");
  let addr = server.local_addr().expect("local_addr");
  assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
  assert_ne!(addr.port(), 0, "OS should assign a real ephemeral port");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_specific_port_is_reachable_on_loopback() {
  // Drop the listener manually by serving with an immediately-cancelled token
  // so the port is freed before the next test reuses it.
  let server = TaskWebServer::bind(0).await.expect("bind");
  let addr = server.local_addr().expect("local_addr");
  let token = CancellationToken::new();
  let serve_token = token.clone();
  let handle = tokio::spawn(async move {
    let _ = server.serve(serve_token).await;
  });
  tokio::time::sleep(Duration::from_millis(80)).await;

  // A TCP connect to the loopback addr must succeed.
  let stream = TcpStream::connect_timeout(&addr.into(), Duration::from_millis(500));
  assert!(stream.is_ok(), "listener not reachable on loopback");

  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn graceful_shutdown_closes_listener_and_drains() {
  let server = TaskWebServer::bind(0).await.expect("bind");
  let addr = server.local_addr().expect("local_addr");
  let token = CancellationToken::new();
  let serve_token = token.clone();
  let handle = tokio::spawn(async move {
    let _ = server.serve(serve_token).await;
  });
  tokio::time::sleep(Duration::from_millis(80)).await;

  token.cancel();
  // serve() must resolve promptly after cancellation.
  tokio::time::timeout(Duration::from_millis(800), handle)
    .await
    .expect("serve did not shut down within timeout")
    .expect("join error");

  // After shutdown, new connections must be refused (or any write fails),
  // proving the listener is gone.
  let stream = TcpStream::connect_timeout(&addr.into(), Duration::from_millis(300));
  match stream {
    Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => { /* expected */ }
    Ok(mut s) => {
      let result = s.write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n");
      assert!(
        result.is_err(),
        "write should fail after shutdown, but succeeded"
      );
    }
    Err(other) => panic!("unexpected connect error after shutdown: {other:?}"),
  }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn open_sse_connection_closes_on_shutdown() {
  // An open EventSource/SSE connection must be closed when the server shuts
  // down. Before the stream observed the cancellation token, Axum's graceful
  // shutdown would hang forever waiting for the in-flight SSE response. This
  // test proves serve() resolves promptly even with an open SSE consumer.
  let server = TaskWebServer::bind(0).await.expect("bind");
  let addr = server.local_addr().expect("local_addr");
  let token = CancellationToken::new();
  let serve_token = token.clone();
  let serve_handle = tokio::spawn(async move {
    let _ = server
      .serve_with_source(super::ScriptedTaskEventSource::new(16), serve_token)
      .await;
  });
  tokio::time::sleep(Duration::from_millis(80)).await;

  // Open an SSE connection and leave it open (read slowly).
  let conn = std::net::TcpStream::connect_timeout(&addr.into(), Duration::from_millis(500))
    .expect("connect to SSE");
  conn.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
  let mut sse = conn;
  sse
    .write_all(b"GET /api/tasks/events HTTP/1.1\r\nHost: x\r\n\r\n")
    .unwrap();
  // Read a little to confirm the stream started serving.
  let mut buf = [0u8; 64];
  let _ = sse.read(&mut buf);

  // Cancel; serve() must resolve promptly despite the open SSE.
  token.cancel();
  tokio::time::timeout(Duration::from_millis(1000), serve_handle)
    .await
    .expect("serve() hung with an open SSE connection (graceful shutdown did not complete)")
    .expect("join error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bind_address_in_use_returns_io_error() {
  // Bind two listeners on the same explicit port; the second must fail with
  // AddrInUse. Use a non-zero port unlikely to collide with other tests by
  // binding the first on port 0, reading its port, then re-binding that port.
  let first = TaskWebServer::bind(0).await.expect("first bind");
  let port = first.local_addr().expect("local_addr").port();
  let second = TaskWebServer::bind(port).await;
  assert!(
    second.is_err(),
    "second bind on occupied port {port} should fail"
  );
  let err = second.unwrap_err();
  assert_eq!(
    err.kind(),
    std::io::ErrorKind::AddrInUse,
    "expected AddrInUse, got {err:?}"
  );
}
