// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Router integration tests: assert static routes return the expected status,
//! content type, key UI markers, non-cache headers, and that the SSE route is
//! GET-only. Uses the real loopback listener on an ephemeral port with a
//! scripted source and raw HTTP over std::net so no extra HTTP-client crate is
//! required.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::ScriptedTaskEventSource;
use super::TaskWebServer;

/// Read until the connection is closed (for static bodies) or a timeout elapses
/// (for SSE). Returns the raw response bytes.
fn http_get(addr: &str, path: &str, read_deadline_ms: u64) -> String {
  let mut stream = TcpStream::connect(addr).expect("connect");
  stream
    .set_read_timeout(Some(Duration::from_millis(read_deadline_ms)))
    .unwrap();
  let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(req.as_bytes()).unwrap();
  let mut buf = Vec::new();
  let _ = stream.read_to_end(&mut buf);
  String::from_utf8_lossy(&buf).to_string()
}

/// Send a non-GET method and read the immediate response.
fn http_method(addr: &str, method: &str, path: &str) -> String {
  let mut stream = TcpStream::connect(addr).expect("connect");
  stream
    .set_read_timeout(Some(Duration::from_millis(400)))
    .unwrap();
  let req = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
  stream.write_all(req.as_bytes()).unwrap();
  let mut buf = Vec::new();
  let _ = stream.read_to_end(&mut buf);
  String::from_utf8_lossy(&buf).to_string()
}

fn header<'a>(resp: &'a str, name: &str) -> Option<&'a str> {
  let lower = name.to_ascii_lowercase();
  resp.lines().take_while(|l| !l.is_empty()).find_map(|l| {
    let l = l
      .strip_prefix(|c: char| c.is_ascii_alphabetic() || c == '-')
      .unwrap_or(l);
    // crude case-insensitive header match
    let line_lower = l.to_ascii_lowercase();
    if line_lower.starts_with(&format!("{lower}:")) {
      Some(l.splitn(2, ':').nth(1).unwrap_or("").trim())
    } else {
      None
    }
  })
}

fn status_line(resp: &str) -> &str {
  resp.lines().next().unwrap_or("")
}

async fn start_server(
  source: ScriptedTaskEventSource,
) -> (String, CancellationToken, tokio::task::JoinHandle<()>) {
  let server = TaskWebServer::bind(0).await.expect("bind");
  let addr = server.local_addr().expect("local_addr");
  let token = CancellationToken::new();
  let serve_token = token.clone();
  let handle = tokio::spawn(async move {
    let _ = server.serve_with_source(source, serve_token).await;
  });
  tokio::time::sleep(Duration::from_millis(80)).await;
  (format!("127.0.0.1:{}", addr.port()), token, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn static_routes_have_expected_status_content_type_and_markers() {
  let source = ScriptedTaskEventSource::new(16);
  let (addr, token, handle) = start_server(source).await;

  let html = http_get(&addr, "/", 600);
  assert!(
    status_line(&html).contains("200"),
    "index status: {}",
    status_line(&html)
  );
  assert!(html.contains("text/html"), "index content-type: {html}");
  assert!(
    html.contains("Task Diagnostics"),
    "index title marker missing"
  );
  assert!(
    html.contains("/app.js"),
    "index must reference same-origin app.js"
  );

  let css = http_get(&addr, "/app.css", 600);
  assert!(status_line(&css).contains("200"));
  assert!(css.contains("text/css"), "css content-type: {css}");
  assert!(css.contains("--detached"), "css detached marker missing");

  let js = http_get(&addr, "/app.js", 600);
  assert!(status_line(&js).contains("200"));
  assert!(js.contains("text/javascript"), "js content-type: {js}");
  assert!(js.contains("EventSource"), "js must use EventSource");
  assert!(
    !js.contains("http://") && !js.contains("https://"),
    "js must make no external requests"
  );

  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn static_routes_are_non_cacheable() {
  let source = ScriptedTaskEventSource::new(16);
  let (addr, token, handle) = start_server(source).await;

  for path in ["/", "/app.css", "/app.js"] {
    let resp = http_get(&addr, path, 600);
    let cc = header(&resp, "cache-control").unwrap_or("");
    assert!(
      cc.to_ascii_lowercase().contains("no-store"),
      "{path} cache-control should be no-store, got {cc:?}"
    );
  }

  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_route_is_get_only() {
  let source = ScriptedTaskEventSource::new(16);
  let (addr, token, handle) = start_server(source).await;

  // POST to an SSE/static route must not succeed as 200 (404/405 acceptable).
  let post = http_method(&addr, "POST", "/api/tasks/events");
  assert!(
    !status_line(&post).contains("200"),
    "POST to SSE must not be 200: {}",
    status_line(&post)
  );

  let put = http_method(&addr, "PUT", "/");
  assert!(
    !status_line(&put).contains("200"),
    "PUT to index must not be 200: {}",
    status_line(&put)
  );

  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sse_route_emits_initial_reset() {
  let source = ScriptedTaskEventSource::new(16);
  source.register_live("root-1/loop", false);
  let (addr, token, handle) = start_server(source).await;

  let resp = http_get(&addr, "/api/tasks/events", 400);
  assert!(
    resp.contains("text/event-stream"),
    "expected event-stream content type: {resp}"
  );
  assert!(
    resp.contains("event:reset"),
    "expected an initial reset event: {resp}"
  );
  assert!(
    resp.contains("root-1/loop"),
    "reset must include the live task: {resp}"
  );

  token.cancel();
  let _ = tokio::time::timeout(Duration::from_millis(500), handle).await;
}
