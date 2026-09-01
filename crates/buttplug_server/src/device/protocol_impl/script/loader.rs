// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Loader for rhai protocol scripts: scans a directory, compiles and
//! validates each `*.rhai` file, and reports per-file outcomes.
//!
//! Failure policy: a missing directory is not an error (nothing is loaded);
//! an unreadable or non-directory path is an error; any per-file failure is
//! fail-soft — the file is skipped with a structured reason and loading
//! continues.

use std::path::{Path, PathBuf};

use rhai::{AST, CallFnOptions, Dynamic, Scope};

#[cfg(test)]
use super::handler::ScriptedProtocolHandler;
use super::{engine::script_engine, handler::ScriptedProtocolFactory};

/// A successfully loaded script protocol.
#[derive(Debug, Clone)]
pub struct LoadedProtocol {
  /// Protocol name reported by the script's `metadata()`.
  pub name: String,
  /// File the protocol was loaded from.
  pub source_path: PathBuf,
  /// Factory producing per-connection handlers for this protocol.
  pub factory: std::sync::Arc<ScriptedProtocolFactory>,
}

impl LoadedProtocol {
  /// Test-only: build a handler instance for this protocol (fresh state).
  #[cfg(test)]
  pub(crate) fn handler_for_test(&self) -> std::sync::Arc<ScriptedProtocolHandler> {
    self.factory.handler()
  }
}

/// A script file that was skipped, with the reason.
#[derive(Debug, Clone)]
pub struct SkippedScript {
  pub source_path: PathBuf,
  pub reason: String,
}

/// Structured outcome of loading a script protocol directory.
#[derive(Debug, Default, Clone)]
pub struct ScriptLoadReport {
  pub loaded: Vec<LoadedProtocol>,
  pub skipped: Vec<SkippedScript>,
}

/// The script protocol API version this build implements.
const SUPPORTED_API_VERSION: i64 = 1;

/// Maximum number of script files loaded from a single directory; excess
/// files (in sorted order) are skipped with a reason.
const MAX_SCRIPT_FILES: usize = 256;

/// Maximum size of a single script file, in bytes.
const MAX_SCRIPT_FILE_SIZE: u64 = 1024 * 1024;

/// Loads all `*.rhai` protocol scripts from `directory`.
///
/// Returns `Err` only when the directory itself cannot be read (missing
/// directories are treated as "nothing to load"). Individual files that fail
/// to compile or violate the script contract are skipped and reported.
pub fn load_script_protocols(directory: &Path) -> Result<ScriptLoadReport, String> {
  let dir_metadata = match std::fs::metadata(directory) {
    Ok(metadata) => metadata,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      // Missing directory: nothing to load, not an error.
      return Ok(ScriptLoadReport::default());
    }
    Err(e) => {
      return Err(format!(
        "cannot access script protocol directory {}: {e}",
        directory.display()
      ));
    }
  };
  if !dir_metadata.is_dir() {
    return Err(format!(
      "script protocol path {} is not a directory",
      directory.display()
    ));
  }

  // Sort the file list so duplicate-name resolution ("first file wins") is
  // deterministic across platforms. Entry-level inspection failures are
  // recorded as skips rather than swallowed, so no file can silently
  // disappear from the report.
  let mut script_files: Vec<PathBuf> = vec![];
  let mut entry_skips: Vec<SkippedScript> = vec![];
  match std::fs::read_dir(directory) {
    Ok(entries) => {
      for entry in entries {
        let entry = match entry {
          Ok(entry) => entry,
          Err(e) => {
            entry_skips.push(SkippedScript {
              source_path: directory.to_owned(),
              reason: format!("error while reading directory entry: {e}"),
            });
            continue;
          }
        };
        let path = entry.path();
        match entry.metadata() {
          Ok(metadata) => {
            if metadata.is_file()
              && path
                .extension()
                .is_some_and(|extension| extension == "rhai")
            {
              script_files.push(path);
            }
          }
          Err(e) => {
            entry_skips.push(SkippedScript {
              source_path: path,
              reason: format!("cannot inspect directory entry: {e}"),
            });
          }
        }
      }
    }
    Err(e) => {
      return Err(format!(
        "cannot read script protocol directory {}: {e}",
        directory.display()
      ));
    }
  }
  script_files.sort();
  // Cap the number of loaded files so a runaway directory cannot push
  // startup cost without bound; the excess (in sorted order) is skipped.
  if script_files.len() > MAX_SCRIPT_FILES {
    for path in script_files.split_off(MAX_SCRIPT_FILES) {
      entry_skips.push(SkippedScript {
        source_path: path,
        reason: format!(
          "script file count exceeds the maximum of {MAX_SCRIPT_FILES}; excess file skipped"
        ),
      });
    }
  }

  let engine = script_engine();
  let mut report = ScriptLoadReport::default();
  report.skipped.extend(entry_skips);
  for script_file in script_files {
    match load_script_file(engine, &script_file) {
      Ok(loaded) => {
        if let Some(existing) = report.loaded.iter().find(|l| l.name == loaded.name) {
          report.skipped.push(SkippedScript {
            source_path: script_file,
            reason: format!(
              "duplicate protocol name {:?} (already loaded from {})",
              loaded.name,
              existing.source_path.display()
            ),
          });
        } else {
          report.loaded.push(loaded);
        }
      }
      Err(reason) => {
        report.skipped.push(SkippedScript {
          source_path: script_file,
          reason,
        });
      }
    }
  }
  Ok(report)
}

/// Compiles and validates a single script file.
fn load_script_file(engine: &rhai::Engine, path: &Path) -> Result<LoadedProtocol, String> {
  let file_metadata =
    std::fs::metadata(path).map_err(|e| format!("cannot read file metadata: {e}"))?;
  if file_metadata.len() > MAX_SCRIPT_FILE_SIZE {
    return Err(format!(
      "script file is larger than the maximum of {MAX_SCRIPT_FILE_SIZE} bytes"
    ));
  }
  let source = std::fs::read_to_string(path).map_err(|e| format!("cannot read file: {e}"))?;

  let ast: AST = engine
    .compile(&source)
    .map_err(|e| format!("parse error: {e}"))?;

  let has_fn = |name: &str| ast.iter_functions().any(|f| f.name == name);

  // metadata() is required.
  if !has_fn("metadata") {
    return Err("script has no metadata() function".to_owned());
  }
  let metadata = call_script_function(engine, &ast, "metadata", vec![])
    .map_err(|e| format!("metadata() failed: {e}"))?;

  let metadata_map = metadata
    .flatten_clone()
    .try_cast::<rhai::Map>()
    .ok_or_else(|| "metadata() must return an object map".to_owned())?;

  let protocol_name = metadata_map
    .get("protocol")
    .and_then(|value| value.as_immutable_string_ref().ok())
    .map(|s| s.to_string())
    .ok_or_else(|| "metadata() is missing string field \"protocol\"".to_owned())?;
  if protocol_name.is_empty() {
    return Err("metadata() field \"protocol\" must not be empty".to_owned());
  }

  let api_version = metadata_map
    .get("api_version")
    .and_then(|value| value.as_int().ok())
    .ok_or_else(|| "metadata() is missing integer field \"api_version\"".to_owned())?;
  if api_version != SUPPORTED_API_VERSION {
    return Err(format!(
      "metadata() api_version {api_version} is not supported (this build supports {SUPPORTED_API_VERSION})"
    ));
  }

  // init_state() is optional; when present it must return a map, and it runs
  // once here under the same operation limits as handlers. The template is
  // also validated to contain only value types that clone deeply, so the
  // per-connection copies handed to handlers can never alias each other
  // through shared cells or captured closure environments.
  let state_template = if has_fn("init_state") {
    let state = call_script_function(engine, &ast, "init_state", vec![])
      .map_err(|e| format!("init_state() failed: {e}"))?;
    if !state.is_map() {
      return Err("init_state() must return an object map".to_owned());
    }
    validate_state_value(&state, "state")?;
    state
  } else {
    Dynamic::from(rhai::Map::new())
  };

  Ok(LoadedProtocol {
    factory: std::sync::Arc::new(ScriptedProtocolFactory::new(
      &protocol_name,
      std::sync::Arc::new(ast),
      state_template,
    )),
    name: protocol_name,
    source_path: path.to_owned(),
  })
}

/// Calls a script function with the engine's limits in force, without
/// evaluating the AST body.
fn call_script_function(
  engine: &rhai::Engine,
  ast: &AST,
  fn_name: &str,
  args: Vec<Dynamic>,
) -> Result<Dynamic, String> {
  let mut scope = Scope::new();
  let mut options = CallFnOptions::new();
  options.eval_ast = false;
  engine
    .call_fn_with_options::<Dynamic>(options, &mut scope, ast, fn_name, args)
    .map_err(|e| e.to_string())
}

/// Recursively validates that a state template only contains value types
/// that clone deeply (integers, floats, bools, chars, strings, Blobs, arrays,
/// and maps thereof).
///
/// Anything else — function pointers, shared cells, timestamps, or any other
/// exotic value — is rejected at load time. This is what makes the
/// per-connection deep-copy guarantee sound: once a template contains only
/// these types, cloning it can never leave two connections aliasing the same
/// underlying value.
fn validate_state_value(value: &Dynamic, path: &str) -> Result<(), String> {
  if value.is_unit()
    || value.is_bool()
    || value.is_int()
    || value.is_float()
    || value.is_char()
    || value.is_string()
    || value.is_blob()
  {
    return Ok(());
  }
  if let Ok(array) = value.as_array_ref() {
    for (index, element) in array.iter().enumerate() {
      validate_state_value(element, &format!("{path}[{index}]"))?;
    }
    return Ok(());
  }
  if let Ok(map) = value.as_map_ref() {
    for (key, element) in map.iter() {
      validate_state_value(element, &format!("{path}.{key}"))?;
    }
    return Ok(());
  }
  Err(format!(
    "init_state() contains an unsupported value of type {} at {path} (allowed: integers, floats, bools, chars, strings, Blobs, arrays, and maps)",
    value.type_name()
  ))
}
