// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use argh::FromArgs;
use tracing_subscriber::EnvFilter;

/// Buttplug client conformance test harness.
///
/// Runs a WebSocket server that drives connecting clients through scripted
/// protocol test sequences, validating correctness against the real Buttplug
/// server implementation.
#[derive(FromArgs)]
struct Args {
  /// websocket listen port
  #[argh(option, default = "12345")]
  port: u16,

  /// output format: stdout, json
  #[argh(option, default = "\"stdout\".to_owned()")]
  format: String,

  /// run only the named sequence (omit for all)
  #[argh(option)]
  sequence: Option<String>,

  /// default per-step timeout in milliseconds
  #[argh(option, default = "5000")]
  timeout: u64,
}

fn main() {
  // Initialize tracing — respects RUST_LOG env var, defaults to info
  tracing_subscriber::fmt()
    .with_env_filter(
      EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
    )
    .init();

  let args: Args = argh::from_env();

  println!("Buttplug Client Conformance Test Harness");
  println!("  Port:     {}", args.port);
  println!("  Format:   {}", args.format);
  println!("  Sequence: {}", args.sequence.as_deref().unwrap_or("all"));
  println!("  Timeout:  {}ms", args.timeout);
  println!();
  println!("Test runner not yet implemented.");
}
