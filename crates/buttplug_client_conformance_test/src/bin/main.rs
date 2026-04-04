// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

use argh::FromArgs;
use buttplug_client_conformance_test::report::Report;
use buttplug_client_conformance_test::runner::run_sequence;
use buttplug_client_conformance_test::step::TestSequence;
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

#[tokio::main]
async fn main() {
  // Initialize tracing — respects RUST_LOG env var, defaults to info
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();

  let args: Args = argh::from_env();

  println!("Buttplug Client Conformance Test Harness");
  println!("  Port:     {}", args.port);
  println!("  Format:   {}", args.format);
  println!("  Sequence: {}", args.sequence.as_deref().unwrap_or("all"));
  println!("  Timeout:  {}ms", args.timeout);
  println!();

  // Placeholder: create a minimal handshake-only sequence
  let sequences: Vec<TestSequence> = vec![
    /* will be populated in Phase 4 */
  ];

  // Filter by --sequence if provided
  let sequences_to_run: Vec<_> = if let Some(ref name) = args.sequence {
    sequences.iter().filter(|s| s.name == name.as_str()).collect()
  } else {
    sequences.iter().collect()
  };

  let mut report = Report::new();

  for sequence in sequences_to_run {
    println!("=== Waiting for client connection for: {} ===", sequence.name);
    println!("    Connect to ws://127.0.0.1:{}", args.port);
    let result = run_sequence(sequence, args.port, args.timeout).await;
    report.add_result(result);
  }

  // Output report
  match args.format.as_str() {
    "json" => println!("{}", report.format_json()),
    _ => println!("{}", report.format_stdout()),
  }

  // Exit with appropriate code
  std::process::exit(if report.all_passed() { 0 } else { 1 });
}
