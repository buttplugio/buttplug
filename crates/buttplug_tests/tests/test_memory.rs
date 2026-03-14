use buttplug_server_device_config::load_protocol_configs;
use humansize::{BINARY, format_size};
use std::alloc::System;
use std::cell::Cell as TlsCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tabled::settings::object::Rows;
use tabled::settings::style::HorizontalLine;
use tabled::{
  builder::Builder,
  settings::{
    Alignment,
    Modify,
    Style,
    object::{Cell, Columns},
    span::ColumnSpan,
  },
};
use tracing::{Level, span};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;

/// Run `$body` inside a named TRACE span.
/// Sync expr:  `in_span!("name", expr)`
/// Sync block:  `in_span!("name", { ... })`
/// Async: `in_span!("name", async { ... }).await`
macro_rules! in_span {
  ($name:expr, async $body:block) => {{
    use tracing::Instrument;
    async $body.instrument(span!(Level::TRACE, $name))
  }};
  ($name:expr, { $($body:tt)* }) => {{
    let _span = span!(Level::TRACE, $name).entered();
    $($body)*
  }};
  ($name:expr, $body:expr) => {{
    let _span = span!(Level::TRACE, $name).entered();
    $body
  }};
}

#[derive(Debug, Clone)]
struct Stats {
  alloc: i64,
  dealloc: i64,
}

impl Stats {
  fn delta(&self) -> i64 {
    self.alloc - self.dealloc
  }
}

impl std::ops::Add for Stats {
  type Output = Self;

  fn add(self, rhs: Self) -> Self::Output {
    Self {
      alloc: self.alloc + rhs.alloc,
      dealloc: self.dealloc + rhs.dealloc,
    }
  }
}

// ---------------------------------------------------------------------------
// Global allocator with per-span memory tracking
// ---------------------------------------------------------------------------

#[global_allocator]
static GLOBAL: Allocator = Allocator {};

struct Allocator;

// Fixed-size span ID stack stored in TLS – no heap allocation needed.
const STACK_DEPTH: usize = 64;

thread_local! {
  /// Set to true while we are inside span-stat bookkeeping to prevent re-entrant tracking.
  static TRACKING: TlsCell<bool> = TlsCell::new(false);
  /// Stack of active span IDs (index 0 = oldest, depth-1 = innermost).
  static SPAN_STACK: TlsCell<[u64; STACK_DEPTH]> = TlsCell::new([0u64; STACK_DEPTH]);
  static SPAN_DEPTH: TlsCell<usize> = TlsCell::new(0);
}

/// RAII guard that holds the TRACKING flag for its lifetime.
/// Obtain one via `TrackingGuard::acquire()`; returns `None` when already tracking.
struct TrackingGuard;

impl TrackingGuard {
  fn acquire() -> Option<Self> {
    TRACKING.with(|t| {
      if t.get() {
        None
      } else {
        t.set(true);
        Some(TrackingGuard)
      }
    })
  }
}

impl Drop for TrackingGuard {
  fn drop(&mut self) {
    TRACKING.with(|t| t.set(false));
  }
}

fn push_span(id: u64) {
  SPAN_DEPTH.with(|d| {
    let depth = d.get();
    if depth < STACK_DEPTH {
      SPAN_STACK.with(|s| {
        let mut arr = s.get();
        arr[depth] = id;
        s.set(arr);
      });
    }
    d.set(depth + 1);
  });
}

fn pop_span(id: u64) {
  SPAN_DEPTH.with(|d| {
    let depth = d.get();
    if depth > 0 {
      SPAN_STACK.with(|s| {
        let arr = s.get();
        if arr[depth - 1] == id {
          d.set(depth - 1);
        }
      });
    }
  });
}

fn current_span() -> Option<u64> {
  SPAN_DEPTH.with(|d| {
    let depth = d.get();
    if depth == 0 {
      return None;
    }
    SPAN_STACK.with(|s| {
      let id = s.get()[depth - 1];
      if id == 0 { None } else { Some(id) }
    })
  })
}

// ---------------------------------------------------------------------------
// Per-span statistics
// ---------------------------------------------------------------------------

struct SpanStats {
  name: String,
  parent: Option<u64>,
  own: Stats,
}

impl SpanStats {
  fn total(reg: &SpanRegistry, id: u64) -> Stats {
    let s = &reg.stats[&id];
    let mut total = s.own.clone();
    for &child_id in &reg.order {
      if reg.stats[&child_id].parent == Some(id) {
        total = total + SpanStats::total(reg, child_id);
      }
    }
    total
  }
}

struct SpanRegistry {
  stats: HashMap<u64, SpanStats>,
  /// Insertion order – used when printing the tree.
  order: Vec<u64>,
}

impl SpanRegistry {
  fn new() -> Self {
    Self {
      stats: HashMap::new(),
      order: Vec::new(),
    }
  }
}

static REGISTRY: OnceLock<Mutex<SpanRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<SpanRegistry> {
  REGISTRY.get_or_init(|| Mutex::new(SpanRegistry::new()))
}

// ---------------------------------------------------------------------------
// GlobalAlloc impl
// ---------------------------------------------------------------------------

unsafe impl std::alloc::GlobalAlloc for Allocator {
  unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
    let ptr = unsafe { System.alloc(layout) };

    if let Some(_guard) = TrackingGuard::acquire() {
      if let Some(span_id) = current_span() {
        if let Ok(mut reg) = registry().try_lock() {
          if let Some(s) = reg.stats.get_mut(&span_id) {
            s.own.alloc += layout.size() as i64;
          }
        }
      }
    }

    ptr
  }

  unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
    unsafe { System.dealloc(ptr, layout) };

    if let Some(_guard) = TrackingGuard::acquire() {
      if let Some(span_id) = current_span() {
        if let Ok(mut reg) = registry().try_lock() {
          if let Some(s) = reg.stats.get_mut(&span_id) {
            s.own.dealloc += layout.size() as i64;
          }
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Tracing layer – captures span lifecycle to drive the ID stack
// ---------------------------------------------------------------------------

struct MemTrackingLayer;

impl<S: tracing::Subscriber> Layer<S> for MemTrackingLayer {
  fn on_new_span(
    &self,
    attrs: &span::Attributes<'_>,
    id: &tracing::span::Id,
    _ctx: tracing_subscriber::layer::Context<'_, S>,
  ) {
    let span_id = id.clone().into_u64();
    let parent = current_span();

    // Hold the guard so HashMap's own allocations aren't attributed anywhere.
    let _guard = TrackingGuard::acquire();
    if let Ok(mut reg) = registry().try_lock() {
      reg.order.push(span_id);
      reg.stats.insert(
        span_id,
        SpanStats {
          name: attrs.metadata().name().to_string(),
          parent,
          own: Stats {
            alloc: 0,
            dealloc: 0,
          },
        },
      );
    }
  }

  fn on_enter(&self, id: &tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
    push_span(id.clone().into_u64());
  }

  fn on_exit(&self, id: &tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
    pop_span(id.clone().into_u64());
  }
}

// ---------------------------------------------------------------------------
// Tree printer
// ---------------------------------------------------------------------------

fn format_bytes(bytes: i64) -> String {
  format_size(bytes.unsigned_abs(), BINARY)
}

fn signed_bytes(bytes: i64) -> String {
  let sign = if bytes < 0 { "-" } else { "+" };
  format!("{}{}", sign, format_size(bytes.unsigned_abs(), BINARY))
}

fn collect_rows(reg: &SpanRegistry, parent: Option<u64>, depth: usize, builder: &mut Builder) {
  for &id in &reg.order {
    let s = &reg.stats[&id];
    if s.parent != parent {
      continue;
    }
    let total = SpanStats::total(reg, id);
    builder.push_record([
      format!("{}{}", "    ".repeat(depth), s.name),
      format_bytes(s.own.alloc),
      format_bytes(s.own.dealloc),
      signed_bytes(s.own.delta()),
      format_bytes(total.alloc),
      format_bytes(total.dealloc),
      signed_bytes(total.delta()),
    ]);
    collect_rows(reg, Some(id), depth + 1, builder);
  }
}

fn print_tree(reg: &SpanRegistry) {
  let mut builder = Builder::new();
  builder.push_record(["Span", "Own", "", "", "Total", "", ""]);
  builder.push_record(["", "+", "-", "Δ", "+", "-", "Δ"]);
  collect_rows(reg, None, 0, &mut builder);

  let table = builder
    .build()
    .with(Style::sharp().horizontals([(2, HorizontalLine::inherit(Style::modern()))]))
    .with(Modify::new(Columns::new(1..)).with(Alignment::right()))
    .with(Modify::new(Rows::new(0..2)).with(Alignment::center()))
    .with(Modify::new(Columns::first()).with(Alignment::left()))
    .with(Modify::new(Cell::new(0, 1)).with(ColumnSpan::new(3)))
    .with(Modify::new(Cell::new(0, 4)).with(ColumnSpan::new(3)))
    .to_string();
  println!("{table}");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_memory() {
  let subscriber = tracing_subscriber::registry().with(MemTrackingLayer);
  let _guard = tracing::subscriber::set_default(subscriber);

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .start_paused(true)
    .build()
    .unwrap()
    .block_on(async {
      let _client = memory_test().await;

      let reg = registry().lock().unwrap();
      print_tree(&reg);
    });
}

async fn memory_test() -> buttplug_client::ButtplugClient {
  in_span!("memory_test", {
    let dcm = in_span!("DeviceConfigurationManager", {
      let builder = in_span!(
        "load_protocol_configs",
        load_protocol_configs(&None, &None, false).unwrap()
      );
      let dcm = in_span!(
        "DeviceConfigurationManagerBuilder.finish",
        builder.finish().unwrap()
      );
      dcm
    });

    let manager = in_span!("ServerDeviceManager", {
      let mut builder = in_span!(
        "ServerDeviceManagerBuilder::new",
        buttplug_server::device::ServerDeviceManagerBuilder::new(dcm)
      );

      in_span!(
        "builder.comm_manager(BtlePlugCommunicationManagerBuilder)",
        builder.comm_manager(
          buttplug_server_hwmgr_btleplug::BtlePlugCommunicationManagerBuilder::default()
        )
      );

      in_span!(
        "ServerDeviceManagerBuilder.finish",
        builder.finish().unwrap()
      )
    });

    let server = in_span!("ButtplugServerBuilder", {
      let builder = buttplug_server::ButtplugServerBuilder::new(manager);

      in_span!("ButtplugServerBuilder.finish", builder.finish().unwrap())
    });

    let connector = in_span!("ButtplugClientInProcessConnector", {
      let mut builder = in_span!(
        "ButtplugInProcessClientConnectorBuilder::default",
        buttplug_client_in_process::ButtplugInProcessClientConnectorBuilder::default()
      );

      in_span!(
        "ButtplugClientInProcessConnectorBuilder.server",
        builder.server(server)
      );

      in_span!(
        "ButtplugInProcessClientConnectorBuilder.finish",
        builder.finish()
      )
    });

    in_span!("ButtplugClient", {
      let client = in_span!(
        "ButtplugClient::new",
        buttplug_client::ButtplugClient::new("Memory Test Client")
      );

      in_span!(
        "ButtplugClient.connect",
        client.connect(connector).await.unwrap()
      );

      client
    })
  })
}
