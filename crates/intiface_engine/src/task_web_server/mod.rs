// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Standalone, opt-in, loopback-only HTTP diagnostics server that visualizes
//! the process-global Buttplug task registry in real time.
//!
//! Unlike [`crate::rest_server::IntifaceRestServer`], this server does **not**
//! consume a [`ButtplugServer`] or replace the normal engine listener. It is
//! intended to run concurrently with normal engine operation: bind it once per
//! engine run (outside any per-client reconnect loop), then [`select!`] across
//! owner cancellation and the server-completion future. See
//! [`TaskWebServer::bind`] / [`TaskWebServer::serve`].
//!
//! The wire protocol uses serializable diagnostics-specific DTOs with stable
//! lowercase event/outcome strings. A connection establishes its task-event
//! subscription *before* reading the snapshot, reconciles events that were
//! queued between subscription and snapshot, emits one authoritative `reset`,
//! and then forwards live events. If the registry broadcast receiver lags, the
//! stream re-snapshots and emits a fresh `reset` rather than continuing with a
//! silently stale model.

// This module is a self-contained unit awaiting engine.rs integration (a
// separately-scoped task). Its public API is intentionally complete even though
// nothing references it in a non-test build yet.
#![allow(dead_code)]

mod assets;
mod protocol;
#[cfg(test)]
mod test_source;

pub(crate) use protocol::TaskEventSource;
#[cfg(test)]
pub(crate) use test_source::ScriptedTaskEventSource;

use std::convert::Infallible;
use std::net::SocketAddr;

use axum::response::sse::{Event, KeepAlive};
use axum::{
  Router,
  extract::State,
  http::{HeaderMap, HeaderValue},
  response::{IntoResponse, Response, Sse},
  routing::get,
};
use futures::stream::Stream;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// Loopback address the diagnostics server always binds. Exposed so callers
/// building engine errors can reuse the exact address string.
pub const LOOPBACK_ADDR: [u8; 4] = [127, 0, 0, 1];

/// Builds the GET-only diagnostics router. The shared state carries the event
/// source and the server's cancellation token so the SSE handler can terminate
/// on shutdown (Axum graceful shutdown alone would hang on an open connection).
pub(crate) fn router<S>(source: S, shutdown: CancellationToken) -> Router
where
  S: TaskEventSource + Clone + Send + Sync + 'static,
{
  Router::new()
    .route("/", get(index_html))
    .route("/app.css", get(app_css))
    .route("/app.js", get(app_js))
    .route("/api/tasks/events", get(task_events::<S>))
    .with_state((source, shutdown))
}

/// A bound diagnostics server. Created via [`TaskWebServer::bind`]; driven via
/// [`TaskWebServer::serve`].
///
/// `serve` consumes `self` and runs until `shutdown` is cancelled (graceful) or
/// an unrecoverable serving error occurs. The practical lifecycle for the
/// engine is:
///
/// ```ignore
/// let server = TaskWebServer::bind(port).await?;     // deterministic bind
/// let child = stop_token.child_token();
/// // ... select! across owner cancellation and server.serve(child) ...
/// ```
pub struct TaskWebServer {
  listener: TcpListener,
}

impl std::fmt::Debug for TaskWebServer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("TaskWebServer")
      .field(
        "listener",
        &self
          .listener
          .local_addr()
          .map(|a| a.to_string())
          .unwrap_or_else(|_| "<unknown>".to_owned()),
      )
      .finish()
  }
}

impl TaskWebServer {
  /// Bind the diagnostics listener to `127.0.0.1:<port>`. `port == 0` requests
  /// an ephemeral port from the OS; retrieve it with [`Self::local_addr`].
  ///
  /// Binding is separated from serving so a bind failure (e.g. address-in-use)
  /// is deterministic and can be reported through the engine's existing
  /// structured error path before any sibling listener is started.
  pub async fn bind(port: u16) -> std::io::Result<Self> {
    let addr = SocketAddr::from((LOOPBACK_ADDR, port));
    let listener = TcpListener::bind(addr).await?;
    Ok(Self { listener })
  }

  /// The actual bound address. Useful when `port == 0` was requested so the
  /// engine can log the assigned URL.
  pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
    self.listener.local_addr()
  }

  /// Serve the diagnostics UI and stream until `shutdown` is cancelled or an
  /// unrecoverable serving error occurs. Uses Axum graceful shutdown so an
  /// in-flight SSE connection is drained before the future resolves.
  ///
  /// Always uses the process-global registry as the event source. For tests
  /// that need a scripted source, use [`Self::serve_with_source`].
  pub async fn serve(self, shutdown: CancellationToken) -> std::io::Result<()> {
    self
      .serve_with_source(protocol::registry_source(), shutdown)
      .await
  }

  /// Like [`Self::serve`] but with an injectable event source. Used by tests
  /// with a scripted source; production uses [`Self::serve`].
  pub(crate) async fn serve_with_source<S>(
    self,
    source: S,
    shutdown: CancellationToken,
  ) -> std::io::Result<()>
  where
    S: TaskEventSource + Clone + Send + Sync + 'static,
  {
    let app = router::<S>(source, shutdown.clone());
    let make = app.into_make_service();
    axum::serve(self.listener, make)
      .with_graceful_shutdown(async move { shutdown.cancelled().await })
      .await
  }
}

fn no_cache_headers() -> HeaderMap {
  let mut headers = HeaderMap::new();
  headers.insert(
    axum::http::header::CACHE_CONTROL,
    HeaderValue::from_static("no-store"),
  );
  headers.insert(
    axum::http::header::PRAGMA,
    HeaderValue::from_static("no-cache"),
  );
  headers.insert(
    axum::http::header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
  );
  headers.insert(
    axum::http::header::CONTENT_SECURITY_POLICY,
    HeaderValue::from_static(
      "default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'",
    ),
  );
  headers
}

fn no_cache_response(content_type: &'static str, body: &'static str) -> Response {
  let mut resp = ([(axum::http::header::CONTENT_TYPE, content_type)], body).into_response();
  resp.headers_mut().extend(no_cache_headers());
  resp
}

async fn index_html() -> Response {
  no_cache_response("text/html; charset=utf-8", assets::INDEX_HTML)
}

async fn app_css() -> Response {
  no_cache_response("text/css; charset=utf-8", assets::APP_CSS)
}

async fn app_js() -> Response {
  no_cache_response("text/javascript; charset=utf-8", assets::APP_JS)
}

/// SSE handler. Performs subscribe-before-snapshot bootstrap reconciliation,
/// emits an authoritative `reset`, then forwards live `started`/`ended` events,
/// re-snapshotting and emitting a fresh `reset` on broadcast lag. The stream
/// observes the server cancellation token so it terminates promptly on shutdown.
async fn task_events<S>(
  State((source, shutdown)): State<(S, CancellationToken)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>>
where
  S: TaskEventSource + Send + Sync + 'static,
{
  let stream = protocol::diagnostics_stream(source, shutdown);
  Sse::new(stream).keep_alive(KeepAlive::default())
}

#[cfg(test)]
mod listener_tests;
#[cfg(test)]
mod protocol_tests;
#[cfg(test)]
mod router_tests;
