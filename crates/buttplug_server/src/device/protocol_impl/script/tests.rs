// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Unit tests for the rhai script protocol subsystem.
//!
//! Test scripts live in this directory as `tests/*.rhai` fixtures; loader
//! failure fixtures are written to temp dirs at runtime via
//! [`write_script_dir`].

use std::{
  collections::HashMap,
  path::{Path, PathBuf},
  sync::Arc,
};

use buttplug_core::errors::ButtplugDeviceError;
use buttplug_server_device_config::{
  BluetoothLESpecifier,
  DeviceConfigurationManager,
  Endpoint,
  ProtocolCommunicationSpecifier,
  load_protocol_configs,
};
use uuid::Uuid;

use crate::device::{
  hardware::{Hardware, HardwareCommand},
  protocol::ProtocolHandler,
};

use super::{ScriptedProtocolHandler, build_script_protocol_manager, load_script_protocols};

/// Path of the shipped script protocol assets.
fn shipped_scripts_dir() -> PathBuf {
  Path::new(env!("CARGO_MANIFEST_DIR"))
    .join("scripts")
    .join("protocols")
}

/// Writes a set of `(file_name, contents)` scripts into a fresh temp
/// directory and returns its path.
fn write_script_dir(scripts: &[(&str, &str)]) -> PathBuf {
  let dir = std::env::temp_dir().join(format!(
    "buttplug-script-test-{}-{}",
    std::process::id(),
    Uuid::new_v4()
  ));
  std::fs::create_dir_all(&dir).expect("should create script test dir");
  for (name, contents) in scripts {
    std::fs::write(dir.join(name), contents).expect("should write script fixture");
  }
  dir
}

/// Compiles a script from source into a handler for a protocol name.
fn handler_from_source(source: &str) -> ScriptedProtocolHandler {
  let engine = super::engine::script_engine();
  let ast = Arc::new(engine.compile(source).expect("test script should compile"));
  ScriptedProtocolHandler::new("testproto", ast, rhai::Dynamic::from(rhai::Map::new()))
}

/// AC.1: scripts register; same-name scripts override built-ins; new-name
/// scripts add new protocols. Drives the full specializer → identify →
/// initialize → command path.
#[tokio::test]
async fn script_loader_registers_and_overrides() {
  // A script that claims the built-in "aneros" name but returns distinctive
  // bytes, plus a script with a novel protocol name.
  let aneros_override = r#"
fn metadata() {
  #{ "protocol": "aneros", "api_version": 1 }
}
fn handle_vibrate(index, speed) {
  [ #{ "endpoint": "tx", "data": [0xAA, 0xBB] } ]
}
"#;
  let novel = r#"
fn metadata() {
  #{ "protocol": "scriptnovel", "api_version": 1 }
}
fn handle_vibrate(index, speed) {
  [ #{ "endpoint": "tx", "data": [0x12, 0x34, speed & 0xff] } ]
}
"#;
  let dir = write_script_dir(&[
    ("a_aneros_override.rhai", aneros_override),
    ("b_novel.rhai", novel),
  ]);

  let protocol_manager = build_script_protocol_manager(Some(&dir)).unwrap();

  // Device config with communication specifiers for both protocols. The
  // aneros entry mirrors the built-in config (names "Massage Demo"); the
  // novel protocol gets its own specifier. A custom base config replaces the
  // internal one entirely, so both must be present.
  let novel_config = serde_json::json!({
    "version": { "major": 5, "minor": 9999 },
    "protocols": {
      "aneros": {
        "communication": [{
          "btle": {
            "names": ["Massage Demo"],
            "services": {
              "0000ff00-0000-1000-8000-00805f9b34fb": {
                "tx": "0000ff01-0000-1000-8000-00805f9b34fb"
              }
            }
          }
        }],
        "defaults": {
          "name": "Aneros Vivi",
          "id": "f023f0f4-6629-469e-84c4-171ed4939f3d",
          "features": [
            {
              "index": 0,
              "id": "a980bc1a-5554-4293-a75f-6d17bf25ebee",
              "output": { "vibrate": { "value": [0, 127] } }
            },
            {
              "index": 1,
              "id": "811d7d6e-6a75-4925-943a-a06042223e3a",
              "output": { "vibrate": { "value": [0, 127] } }
            }
          ]
        }
      },
      "scriptnovel": {
        "communication": [{
          "btle": {
            "names": ["Script Novel Device"],
            "services": {
              "0000ff00-0000-1000-8000-00805f9b34fb": {
                "tx": "0000ff01-0000-1000-8000-00805f9b34fb"
              }
            }
          }
        }],
        "defaults": {
          "name": "Script Novel Device",
          "id": "e5f68425-83a5-4b0e-a45f-9dcb27e0a111",
          "features": [{
            "index": 0,
            "id": "1f32950f-97e5-4f6f-b20f-56c3ac6b2222",
            "output": { "vibrate": { "value": [0, 100] } }
          }]
        }
      }
    }
  });
  let dcm: DeviceConfigurationManager =
    load_protocol_configs(&Some(novel_config.to_string()), &None, true)
      .unwrap()
      .finish()
      .unwrap();

  async fn drive_protocol(
    protocol_manager: &crate::device::protocol::ProtocolManager,
    dcm: &DeviceConfigurationManager,
    protocol: &str,
    device_name: &str,
  ) -> Vec<HardwareCommand> {
    let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
      BluetoothLESpecifier::new_from_device(device_name, &HashMap::new(), &[]),
    );
    let specializers = protocol_manager.protocol_specializers(
      &specifier,
      dcm.base_communication_specifiers(),
      dcm.user_communication_specifiers(),
    );
    assert!(
      !specializers.is_empty(),
      "expected a specializer for {protocol}"
    );
    let mut identifier = specializers.into_iter().next().unwrap().identify();
    let hardware = Arc::new(Hardware::new(
      device_name,
      "address",
      &[Endpoint::Tx],
      &None,
      false,
      Box::new(crate::device::hardware::simulated::SimulatedHardwareInternal::new("address")),
    ));
    let (device_identifier, mut initializer) = identifier
      .identify(hardware.clone(), specifier)
      .await
      .unwrap();
    assert_eq!(device_identifier.protocol(), protocol);
    let definition = dcm.device_definition(&device_identifier).unwrap();
    let handler = initializer.initialize(hardware, &definition).await.unwrap();
    handler
      .handle_output_vibrate_cmd(0, Uuid::new_v4(), 50)
      .unwrap()
  }

  // Overridden built-in: distinctive scripted bytes, not the Rust aneros
  // output ([0xF1, speed]).
  let aneros_commands = drive_protocol(&protocol_manager, &dcm, "aneros", "Massage Demo").await;
  let HardwareCommand::Write(aneros_write) = &aneros_commands[0] else {
    panic!("expected a write command");
  };
  assert_eq!(aneros_write.data(), &vec![0xAA, 0xBB]);

  // Novel protocol registers and functions end to end.
  let novel_commands = drive_protocol(
    &protocol_manager,
    &dcm,
    "scriptnovel",
    "Script Novel Device",
  )
  .await;
  let HardwareCommand::Write(write) = &novel_commands[0] else {
    panic!("expected a write command");
  };
  assert_eq!(write.data(), &vec![0x12, 0x34, 50]);

  std::fs::remove_dir_all(&dir).ok();
}

/// AC.2(b): the shipped scripts/protocols directory loads all three
/// protocols with nothing skipped.
#[test]
fn script_assets_load_cleanly() {
  let report = load_script_protocols(&shipped_scripts_dir()).unwrap();
  let mut names: Vec<_> = report.loaded.iter().map(|l| l.name.as_str()).collect();
  names.sort_unstable();
  assert_eq!(names, vec!["aneros", "jejoue", "maxpro"]);
  assert!(
    report.skipped.is_empty(),
    "shipped scripts should not be skipped: {:?}",
    report
      .skipped
      .iter()
      .map(|s| (&s.source_path, &s.reason))
      .collect::<Vec<_>>()
  );
}

/// AC.4: per-file failures are skipped with structured reasons; remaining
/// scripts still load.
#[test]
fn script_loader_fail_soft() {
  let syntax_error = "fn metadata( { #{ }"; // unparseable
  let missing_metadata = "fn handle_vibrate(i, s) { [] }";
  let unknown_api_version = r#"
fn metadata() { #{ "protocol": "badversion", "api_version": 99 } }
"#;
  let init_state_throws = r#"
fn metadata() { #{ "protocol": "thrower", "api_version": 1 } }
fn init_state() { throw "boom"; }
"#;
  let init_state_not_map = r#"
fn metadata() { #{ "protocol": "notmap", "api_version": 1 } }
fn init_state() { 42 }
"#;
  let good = r#"
fn metadata() { #{ "protocol": "goodone", "api_version": 1 } }
fn handle_vibrate(i, s) { [ #{ "endpoint": "tx", "data": [1] } ] }
"#;
  let dir = write_script_dir(&[
    ("a_syntax_error.rhai", syntax_error),
    ("b_missing_metadata.rhai", missing_metadata),
    ("c_unknown_api_version.rhai", unknown_api_version),
    ("d_init_state_throws.rhai", init_state_throws),
    ("e_init_state_not_map.rhai", init_state_not_map),
    ("f_good.rhai", good),
  ]);

  let report = load_script_protocols(&dir).unwrap();

  assert_eq!(report.loaded.len(), 1, "only the good script should load");
  assert_eq!(report.loaded[0].name, "goodone");
  assert_eq!(report.skipped.len(), 5, "all five bad fixtures skipped");
  let reasons: Vec<String> = report.skipped.iter().map(|s| s.reason.clone()).collect();
  assert!(
    reasons.iter().any(|r| r.contains("parse error")),
    "syntax error reason: {reasons:?}"
  );
  assert!(
    reasons.iter().any(|r| r.contains("metadata")),
    "missing metadata reason: {reasons:?}"
  );
  assert!(
    reasons.iter().any(|r| r.contains("api_version")),
    "api version reason: {reasons:?}"
  );
  assert!(
    reasons
      .iter()
      .any(|r| r.contains("init_state() failed") || r.contains("init_state() must return")),
    "init_state reasons: {reasons:?}"
  );

  // Duplicate protocol names: first (sorted) file wins, later files skipped.
  let dup_a = r#"
fn metadata() { #{ "protocol": "dupname", "api_version": 1 } }
fn handle_vibrate(i, s) { [ #{ "endpoint": "tx", "data": [1] } ] }
"#;
  let dup_b = r#"
fn metadata() { #{ "protocol": "dupname", "api_version": 1 } }
fn handle_vibrate(i, s) { [ #{ "endpoint": "tx", "data": [2] } ] }
"#;
  let dir = write_script_dir(&[("a_first.rhai", dup_a), ("b_second.rhai", dup_b)]);
  let report = load_script_protocols(&dir).unwrap();
  assert_eq!(report.loaded.len(), 1);
  assert_eq!(report.skipped.len(), 1);
  assert!(report.skipped[0].reason.contains("duplicate"));
  assert!(report.skipped[0].source_path.ends_with("b_second.rhai"));

  std::fs::remove_dir_all(&dir).ok();
}

/// AC.4: `import` and `eval` are contract violations and are rejected.
#[test]
fn script_loader_rejects_import_and_eval() {
  let import_script = r#"
import "somewhere" as somewhere;
fn metadata() { #{ "protocol": "importer", "api_version": 1 } }
"#;
  let eval_script = r#"
fn metadata() { #{ "protocol": "evaluator", "api_version": 1 } }
fn handle_vibrate(i, s) { eval("[]") }
"#;
  let good = r#"
fn metadata() { #{ "protocol": "stillgood", "api_version": 1 } }
"#;
  let dir = write_script_dir(&[
    ("a_import.rhai", import_script),
    ("b_eval.rhai", eval_script),
    ("c_good.rhai", good),
  ]);
  let report = load_script_protocols(&dir).unwrap();
  assert_eq!(report.loaded.len(), 1);
  assert_eq!(report.loaded[0].name, "stillgood");
  let skipped_names: Vec<_> = report
    .skipped
    .iter()
    .map(|s| {
      s.source_path
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
    })
    .collect();
  assert!(skipped_names.contains(&"a_import.rhai".to_owned()));
  assert!(skipped_names.contains(&"b_eval.rhai".to_owned()));
  assert!(
    report
      .skipped
      .iter()
      .all(|s| s.reason.contains("parse error")),
    "import/eval rejection should be a parse error: {:?}",
    report.skipped.iter().map(|s| &s.reason).collect::<Vec<_>>()
  );
  std::fs::remove_dir_all(&dir).ok();
}

/// AC.4 (converse): a present-but-not-a-directory path errors loudly.
#[test]
fn script_unreadable_directory_fails_finish() {
  // A file where a directory is expected.
  let file_path =
    std::env::temp_dir().join(format!("buttplug-script-not-a-dir-{}", Uuid::new_v4()));
  std::fs::write(&file_path, "not a directory").unwrap();
  let result = build_script_protocol_manager(Some(&file_path));
  let error_text = match result {
    Ok(_) => panic!("expected a non-directory path to fail"),
    Err(message) => message,
  };
  assert!(error_text.contains("not a directory"), "{error_text}");

  // The builder surfaces this as the pinned server error variant. The file
  // must still exist here; it is removed at the end of the test.
  let dcm = DeviceConfigurationManager::default();
  let mut builder = crate::device::ServerDeviceManagerBuilder::new(dcm);
  builder.script_protocol_directory(file_path.clone());
  match builder.finish() {
    Err(crate::ButtplugServerError::ScriptProtocolLoadError(_)) => {}
    Err(other) => panic!("expected ScriptProtocolLoadError, got {other:?}"),
    Ok(_) => panic!("expected ScriptProtocolLoadError, got Ok"),
  }
  std::fs::remove_file(&file_path).ok();
}

/// AC.5: a throwing handler becomes a DeviceSpecificError with the protocol
/// name prefix, never a panic.
#[test]
fn script_handler_runtime_error() {
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) { throw "kaboom"; }
"#,
  );
  let result = handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20);
  match result {
    Err(ButtplugDeviceError::DeviceSpecificError(message)) => {
      assert!(
        message.starts_with("Rhai protocol testproto: "),
        "message: {message}"
      );
      assert!(message.contains("kaboom"), "message: {message}");
    }
    other => panic!("expected DeviceSpecificError, got {other:?}"),
  }
}

/// AC.5: invalid command shapes/values are rejected with errors naming the
/// problem: out-of-range byte, unknown endpoint string, wrong return shape.
#[test]
fn script_handler_validation_rejects_bad_commands() {
  // Out-of-range byte value.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) { [ #{ "endpoint": "tx", "data": [300] } ] }
"#,
  );
  match handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20) {
    Err(ButtplugDeviceError::DeviceSpecificError(message)) => {
      assert!(message.contains("Rhai protocol testproto: "), "{message}");
      assert!(message.contains("out of range"), "{message}");
    }
    other => panic!("expected DeviceSpecificError, got {other:?}"),
  }

  // Unknown endpoint string.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) { [ #{ "endpoint": "notanendpoint", "data": [1] } ] }
"#,
  );
  match handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20) {
    Err(ButtplugDeviceError::InvalidEndpoint(endpoint)) => {
      assert_eq!(endpoint, "notanendpoint");
    }
    other => panic!("expected InvalidEndpoint, got {other:?}"),
  }

  // Wrong shape: not an array.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) { 42 }
"#,
  );
  match handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20) {
    Err(ButtplugDeviceError::DeviceSpecificError(message)) => {
      assert!(message.contains("array of command maps"), "{message}");
    }
    other => panic!("expected DeviceSpecificError, got {other:?}"),
  }

  // Missing handler → UnhandledCommand (release-mode default policy).
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
"#,
  );
  match handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20) {
    Err(ButtplugDeviceError::UnhandledCommand(_)) => {}
    other => panic!("expected UnhandledCommand, got {other:?}"),
  }
}

/// AC.5: an infinite-loop handler terminates via the operation budget.
#[test]
fn script_handler_op_limit_terminates_loop() {
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) {
  let x = 0;
  while true { x += 1; }
  []
}
"#,
  );
  match handler.handle_output_vibrate_cmd(0, Uuid::new_v4(), 20) {
    Err(ButtplugDeviceError::DeviceSpecificError(message)) => {
      assert!(message.contains("Rhai protocol testproto: "), "{message}");
    }
    other => panic!("expected DeviceSpecificError, got {other:?}"),
  }
}

/// AC.6: per-connection state isolation — two handlers from the same compiled
/// script keep independent `this` state.
#[test]
fn script_state_isolated_per_connection() {
  let source = r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn init_state() { #{ "last": 0 } }
fn handle_vibrate(index, speed) {
  this.last = speed;
  [ #{ "endpoint": "tx", "data": [this.last & 0xff] } ]
}
"#;
  let engine = super::engine::script_engine();
  let ast = Arc::new(engine.compile(source).unwrap());
  let template = {
    let mut scope = rhai::Scope::new();
    let mut options = rhai::CallFnOptions::new();
    options.eval_ast = false;
    engine
      .call_fn_with_options::<rhai::Dynamic>(options, &mut scope, &ast, "init_state", ())
      .unwrap()
  };

  let handler_a = ScriptedProtocolHandler::new("testproto", ast.clone(), template.flatten_clone());
  let handler_b = ScriptedProtocolHandler::new("testproto", ast.clone(), template.flatten_clone());

  let commands_a = handler_a
    .handle_output_vibrate_cmd(0, Uuid::new_v4(), 0x11)
    .unwrap();
  let commands_b = handler_b
    .handle_output_vibrate_cmd(0, Uuid::new_v4(), 0x22)
    .unwrap();

  let data = |commands: &Vec<HardwareCommand>| match &commands[0] {
    HardwareCommand::Write(write) => write.data().clone(),
    _ => panic!("expected write"),
  };
  assert_eq!(data(&commands_a), vec![0x11]);
  assert_eq!(data(&commands_b), vec![0x22]);

  // State persists within a connection: next call sees the stored value.
  let commands_a_next = handler_a
    .handle_output_vibrate_cmd(0, Uuid::new_v4(), 0x33)
    .unwrap();
  assert_eq!(data(&commands_a_next), vec![0x33]);
}

/// AC.6b: command-id semantics — default forwards the feature id; scripts can
/// override with fixed UUIDs (jejoue's protocol UUID).
#[test]
fn script_command_id_default_and_override() {
  let feature_id = Uuid::new_v4();

  // Default: the incoming feature id is forwarded.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) { [ #{ "endpoint": "tx", "data": [1] } ] }
"#,
  );
  let commands = handler
    .handle_output_vibrate_cmd(0, feature_id, 20)
    .unwrap();
  let HardwareCommand::Write(write) = &commands[0] else {
    panic!("expected write");
  };
  assert_eq!(
    write.command_id(),
    &std::collections::HashSet::from([feature_id])
  );

  // Override: fixed protocol UUID.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) {
  [ #{ "endpoint": "tx", "data": [1], "command_ids": ["d3dd2bf5-b029-4bc1-9466-39f82c2e3258"] } ]
}
"#,
  );
  let commands = handler
    .handle_output_vibrate_cmd(0, feature_id, 20)
    .unwrap();
  let HardwareCommand::Write(write) = &commands[0] else {
    panic!("expected write");
  };
  assert_eq!(
    write.command_id(),
    &std::collections::HashSet::from([uuid::uuid!("d3dd2bf5-b029-4bc1-9466-39f82c2e3258")])
  );

  // Invalid UUID in command_ids is rejected.
  let handler = handler_from_source(
    r#"
fn metadata() { #{ "protocol": "testproto", "api_version": 1 } }
fn handle_vibrate(index, speed) {
  [ #{ "endpoint": "tx", "data": [1], "command_ids": ["not-a-uuid"] } ]
}
"#,
  );
  let result = handler.handle_output_vibrate_cmd(0, feature_id, 20);
  match result {
    Err(ButtplugDeviceError::DeviceSpecificError(message)) => {
      assert!(message.contains("not a valid UUID"), "{message}");
    }
    other => panic!("expected DeviceSpecificError, got {other:?}"),
  }
}

/// AC.7(b): feature on, no directory configured — built-in behavior is
/// untouched.
#[test]
fn script_feature_without_directory_is_inert() {
  let manager = build_script_protocol_manager(None).unwrap();
  // The default manager's specializer behavior is exercised through the
  // internal device config: the aneros protocol still resolves.
  let dcm: DeviceConfigurationManager = load_protocol_configs(&None, &None, false)
    .unwrap()
    .finish()
    .unwrap();
  let specifier = ProtocolCommunicationSpecifier::BluetoothLE(
    BluetoothLESpecifier::new_from_device("Massage Demo", &HashMap::new(), &[]),
  );
  let specializers = manager.protocol_specializers(
    &specifier,
    dcm.base_communication_specifiers(),
    dcm.user_communication_specifiers(),
  );
  assert!(!specializers.is_empty());
}

/// Loads one of the shipped protocol scripts as a handler.
fn shipped_handler(protocol: &str) -> std::sync::Arc<ScriptedProtocolHandler> {
  let report = load_script_protocols(&shipped_scripts_dir()).unwrap();
  report
    .loaded
    .iter()
    .find(|l| l.name == protocol)
    .unwrap_or_else(|| panic!("shipped script {protocol} should load"))
    .handler_for_test()
}

/// Extracts the write commands' command-id sets for parity comparison
/// (`HardwareWriteCmd::PartialEq` intentionally ignores command ids).
fn command_ids(commands: &[HardwareCommand]) -> Vec<std::collections::HashSet<Uuid>> {
  commands
    .iter()
    .map(|command| match command {
      HardwareCommand::Write(write) => write.command_id().clone(),
      _ => panic!("expected write command"),
    })
    .collect()
}

/// AC.2(a)/AC.3: the shipped aneros/jejoue/maxpro scripts produce
/// byte-identical hardware writes (including command ids) to the Rust
/// implementations across meaningful input sequences. For jejoue the
/// sequence exercises every pattern-selection branch.
#[test]
fn script_parity_with_rust_impls() {
  use crate::device::protocol_impl::{aneros, jejoue, maxpro};

  let feature_id = Uuid::new_v4();

  // --- aneros: stateless, two indices, several speeds.
  let rust_handler = aneros::Aneros::default();
  let script_handler = shipped_handler("aneros");
  for (index, speed) in [(0, 0u32), (0, 64), (1, 13), (1, 127), (0, 0)] {
    let rust_commands = rust_handler
      .handle_output_vibrate_cmd(index, feature_id, speed)
      .unwrap();
    let script_commands = script_handler
      .handle_output_vibrate_cmd(index, feature_id, speed)
      .unwrap();
    assert_eq!(rust_commands, script_commands, "aneros ({index}, {speed})");
    assert_eq!(
      command_ids(&rust_commands),
      command_ids(&script_commands),
      "aneros command ids ({index}, {speed})"
    );
  }

  // --- maxpro: CRC computation across speeds.
  let rust_handler = maxpro::Maxpro::default();
  let script_handler = shipped_handler("maxpro");
  for speed in [0u32, 1, 50, 100] {
    let rust_commands = rust_handler
      .handle_output_vibrate_cmd(0, feature_id, speed)
      .unwrap();
    let script_commands = script_handler
      .handle_output_vibrate_cmd(0, feature_id, speed)
      .unwrap();
    assert_eq!(rust_commands, script_commands, "maxpro ({speed})");
    assert_eq!(
      command_ids(&rust_commands),
      command_ids(&script_commands),
      "maxpro command ids ({speed})"
    );
  }

  // --- jejoue: stateful; drive all four pattern branches.
  // Branch sequence: (index 1 nonzero from zero) → [3, s1];
  // (index 0 nonzero while 1 active) → [2, s0]; (index 0 back to zero with 1
  // still active) → [3, s1]; (both zero) → [1, 0].
  let rust_handler = jejoue::JeJoue::default();
  let script_handler = shipped_handler("jejoue");
  let sequence = [(0u32, 3u32), (1, 3), (0, 0), (1, 0)];
  for (index, speed) in sequence {
    let rust_commands = rust_handler
      .handle_output_vibrate_cmd(index, feature_id, speed)
      .unwrap();
    let script_commands = script_handler
      .handle_output_vibrate_cmd(index, feature_id, speed)
      .unwrap();
    assert_eq!(rust_commands, script_commands, "jejoue ({index}, {speed})");
    assert_eq!(
      command_ids(&rust_commands),
      command_ids(&script_commands),
      "jejoue command ids ({index}, {speed})"
    );
  }

  // Both-zero stop case from a running state (matches the YAML's final stop).
  let rust_commands = rust_handler
    .handle_output_vibrate_cmd(1, feature_id, 0)
    .unwrap();
  let script_commands = script_handler
    .handle_output_vibrate_cmd(1, feature_id, 0)
    .unwrap();
  assert_eq!(rust_commands, script_commands, "jejoue stop");
}
