// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

//! The shared, hardened rhai engine used by all script protocols.
//!
//! One [`Engine`] instance is created lazily and shared by every
//! [`crate::device::protocol_impl::script`] handler; rhai engines are
//! expensive to build and safe to share when the `sync` feature is on.
//!
//! Hardening applied here (documented in `docs/script-protocols.md`):
//!
//! - Module resolution is a [`DummyModuleResolver`], so `import` can never
//!   load files, and `import`/`eval` are additionally rejected at parse time
//!   via [`Engine::disable_symbol`].
//! - Resource budgets: `max_operations`, `max_call_levels`, `max_array_size`,
//!   `max_map_size`, `max_string_size`. Scripts that exceed a budget terminate
//!   with an error instead of hanging.

use once_cell::sync::Lazy;
use rhai::{Engine, module_resolvers::DummyModuleResolver};

/// Maximum number of executed operations per script function call.
const MAX_OPERATIONS: u64 = 1_000_000;
/// Maximum function call nesting depth (rhai's default, made explicit).
const MAX_CALL_LEVELS: usize = 64;
/// Maximum size of arrays created by scripts.
const MAX_ARRAY_SIZE: usize = 4096;
/// Maximum size of object maps created by scripts.
const MAX_MAP_SIZE: usize = 4096;
/// Maximum length of strings created by scripts.
const MAX_STRING_SIZE: usize = 65_536;

static SCRIPT_ENGINE: Lazy<Engine> = Lazy::new(|| {
  let mut engine = Engine::new();
  // Engine::new() installs a FileModuleResolver on native targets; scripts
  // must never be able to import modules from disk, so replace it with a
  // resolver that can never resolve anything.
  engine.set_module_resolver(DummyModuleResolver::new());
  // No dynamic evaluation and no module imports, enforced at parse time.
  engine.disable_symbol("eval");
  engine.disable_symbol("import");
  // Resource budgets so broken or malicious scripts error out instead of
  // hanging the server.
  engine.set_max_operations(MAX_OPERATIONS);
  engine.set_max_call_levels(MAX_CALL_LEVELS);
  engine.set_max_array_size(MAX_ARRAY_SIZE);
  engine.set_max_map_size(MAX_MAP_SIZE);
  engine.set_max_string_size(MAX_STRING_SIZE);
  engine
});

/// Returns the shared hardened script [`Engine`].
pub(crate) fn script_engine() -> &'static Engine {
  Lazy::force(&SCRIPT_ENGINE)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_eval_is_disabled() {
    let engine = script_engine();
    assert!(engine.compile("let x = eval(\"1 + 1\");").is_err());
  }

  #[test]
  fn test_import_is_disabled() {
    let engine = script_engine();
    assert!(engine.compile("import \"foo\" as foo;").is_err());
  }

  #[test]
  fn test_infinite_loop_terminates() {
    let engine = script_engine();
    let ast = engine
      .compile("fn spin() { let x = 0; while true { x += 1; } }")
      .unwrap();
    let mut scope = rhai::Scope::new();
    let result: Result<rhai::Dynamic, _> = engine.call_fn_with_options(
      {
        let mut options = rhai::CallFnOptions::new();
        options.eval_ast = false;
        options
      },
      &mut scope,
      &ast,
      "spin",
      (),
    );
    assert!(result.is_err());
  }
}
