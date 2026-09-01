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
//! - The std package's `sleep` is replaced with an error: it blocks the
//!   calling thread without consuming operations, so it would bypass every
//!   budget below.
//! - `print`/`debug` output is routed into the server log instead of writing
//!   to stdout.
//! - Resource budgets: `max_operations`, `max_call_levels`, `max_functions`,
//!   `max_variables`, `max_array_size`, `max_map_size`, `max_string_size`.
//!   Scripts that exceed a budget terminate with an error instead of hanging.

use once_cell::sync::Lazy;
use rhai::{Dynamic, Engine, EvalAltResult, Position, module_resolvers::DummyModuleResolver};

/// Maximum number of executed operations per script function call.
const MAX_OPERATIONS: u64 = 1_000_000;
/// Maximum function call nesting depth (rhai's default, made explicit).
const MAX_CALL_LEVELS: usize = 64;
/// Maximum number of functions a script may define.
const MAX_FUNCTIONS: usize = 1024;
/// Maximum number of variables live at once while a script runs.
const MAX_VARIABLES: usize = 1024;
/// Maximum size of arrays created by scripts.
const MAX_ARRAY_SIZE: usize = 4096;
/// Maximum size of object maps created by scripts.
const MAX_MAP_SIZE: usize = 4096;
/// Maximum length of strings created by scripts.
const MAX_STRING_SIZE: usize = 65_536;

/// Error returned by functions that are replaced because they would bypass
/// the engine's execution budgets or break the sandbox.
fn disabled_function_error(message: &str) -> Box<EvalAltResult> {
  EvalAltResult::ErrorRuntime(Dynamic::from(message.to_owned()), Position::NONE).into()
}

static SCRIPT_ENGINE: Lazy<Engine> = Lazy::new(|| {
  let mut engine = Engine::new();
  // Engine::new() installs a FileModuleResolver on native targets; scripts
  // must never be able to import modules from disk, so replace it with a
  // resolver that can never resolve anything.
  engine.set_module_resolver(DummyModuleResolver::new());
  // No dynamic evaluation and no module imports, enforced at parse time.
  engine.disable_symbol("eval");
  engine.disable_symbol("import");
  // The std package's sleep(INT)/sleep(FLOAT) block the calling thread
  // without executing operations, so no budget ever fires while they run.
  // Replace both overloads with an immediate error.
  engine.register_fn(
    "sleep",
    |_seconds: rhai::INT| -> Result<Dynamic, Box<EvalAltResult>> {
      Err(disabled_function_error(
        "sleep is disabled in protocol scripts",
      ))
    },
  );
  engine.register_fn(
    "sleep",
    |_seconds: rhai::FLOAT| -> Result<Dynamic, Box<EvalAltResult>> {
      Err(disabled_function_error(
        "sleep is disabled in protocol scripts",
      ))
    },
  );
  // Keep print/debug inside the server's logging instead of raw stdout.
  engine.on_print(|text| info!("script print: {text}"));
  engine.on_debug(|text, _source, _position| debug!("script debug: {text}"));
  // Resource budgets so broken or malicious scripts error out instead of
  // hanging the server.
  engine.set_max_operations(MAX_OPERATIONS);
  engine.set_max_call_levels(MAX_CALL_LEVELS);
  engine.set_max_functions(MAX_FUNCTIONS);
  engine.set_max_variables(MAX_VARIABLES);
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
    let mut options = rhai::CallFnOptions::new();
    options.eval_ast = false;
    let result: Result<rhai::Dynamic, _> =
      engine.call_fn_with_options(options, &mut scope, &ast, "spin", ());
    assert!(result.is_err());
  }

  #[test]
  fn test_sleep_is_disabled() {
    let engine = script_engine();
    let ast = engine
      .compile("fn nap() { sleep(999999999); }")
      .expect("sleep(…) should still parse");
    let mut scope = rhai::Scope::new();
    let mut options = rhai::CallFnOptions::new();
    options.eval_ast = false;
    let result: Result<rhai::Dynamic, _> =
      engine.call_fn_with_options(options, &mut scope, &ast, "nap", ());
    let error = result.expect_err("sleep should error immediately");
    assert!(
      error.to_string().contains("sleep is disabled"),
      "error: {error}"
    );
  }
}
