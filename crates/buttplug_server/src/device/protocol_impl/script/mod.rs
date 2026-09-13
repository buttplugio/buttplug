// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! Rhai-based script protocol support.
//!
//! Scripts (one protocol per `*.rhai` file) are loaded from a directory on
//! disk, compiled once, and registered in the protocol map alongside the
//! built-in Rust protocols. A script whose protocol name matches a built-in
//! replaces it; a script with a new name registers a new protocol.
//!
//! The script API contract is documented in `docs/script-protocols.md`; the
//! pieces implemented here are:
//!
//! - [`engine`]: the shared, hardened rhai engine.
//! - [`loader`]: directory scan + per-file validation with a structured,
//!   fail-soft report.
//! - [`handler`]: the [`crate::device::protocol::ProtocolHandler`]
//!   implementation backed by script functions, with per-connection `this`
//!   state.

mod engine;
mod handler;
mod loader;
#[cfg(test)]
mod tests;

pub use handler::{ScriptedProtocolFactory, ScriptedProtocolHandler};
pub use loader::{LoadedProtocol, ScriptLoadReport, SkippedScript, load_script_protocols};

use std::{collections::HashMap, path::Path, sync::Arc};

use crate::device::protocol::{ProtocolIdentifierFactory, ProtocolManager};

/// Builds a [`ProtocolManager`] including any script protocols from
/// `directory` (when `Some`).
///
/// - `None` → the default (built-in only) protocol manager.
/// - `Some(dir)` where `dir` does not exist → default protocol manager
///   (info-logged).
/// - `Some(dir)` which exists but cannot be read, or is not a directory →
///   `Err` (the caller surfaces this as a startup error).
///
/// Script protocols override same-name built-ins (info-logged at the point of
/// replacement); per-file script failures are warn-logged and skipped.
/// Logging happens here, at the single call boundary.
pub fn build_script_protocol_manager(directory: Option<&Path>) -> Result<ProtocolManager, String> {
  let Some(directory) = directory else {
    return Ok(ProtocolManager::default());
  };

  let report = load_script_protocols(directory)?;
  if report.loaded.is_empty() && report.skipped.is_empty() {
    info!(
      "No script protocol files found in {}; using built-in protocols only",
      directory.display()
    );
  }
  for skipped in &report.skipped {
    warn!(
      "Skipping script protocol file {}: {}",
      skipped.source_path.display(),
      skipped.reason
    );
  }
  for loaded in &report.loaded {
    info!(
      "Loaded script protocol {} from {}",
      loaded.name,
      loaded.source_path.display()
    );
  }

  let mut protocol_map: HashMap<String, Arc<dyn ProtocolIdentifierFactory>> =
    crate::device::protocol_impl::get_default_protocol_map();
  for loaded in report.loaded {
    if protocol_map
      .insert(loaded.name.clone(), loaded.factory)
      .is_some()
    {
      info!(
        "Script protocol {} overrides built-in protocol of the same name",
        loaded.name
      );
    }
  }
  Ok(ProtocolManager::from_map(protocol_map))
}
