// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[test]
fn sdl_docs_contract_check() {
  let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"));
  let lower = text.to_ascii_lowercase();

  assert!(lower.contains("main-only"), "missing main-only layout");
  assert!(
    lower.contains("trigger-only"),
    "missing trigger-only layout"
  );
  assert!(lower.contains("both"), "missing both-capabilities layout");
  assert!(
    text.contains("SDL Gamepad {instance_id}") || lower.contains("sdl gamepad {instance_id}"),
    "missing deterministic fallback name form"
  );
  assert!(
    lower.contains("neither main rumble nor trigger rumble") && lower.contains("skipped"),
    "missing neither-capability skip policy"
  );
  assert!(
    lower.contains("pure")
      && lower.contains("property queries")
      && lower.contains("later scan")
      && lower.contains("retries"),
    "missing retryable property-probe policy"
  );
  assert!(
    lower.contains("identity is connection-scoped"),
    "missing connection-scoped identity limitation"
  );
  assert!(
    lower.contains("## manual release validation"),
    "missing manual release validation section"
  );
  assert!(
    lower.contains("8 bytes") && lower.contains("four") && lower.contains("logical slots"),
    "missing eight-byte four-slot framing"
  );
  assert!(
    text.contains("--use-sdl-gamepad"),
    "missing --use-sdl-gamepad opt-in flag"
  );
  assert!(
    text.contains("sdl-gamepad-manager"),
    "missing sdl-gamepad-manager opt-in feature"
  );
  assert!(
    lower.contains("simple sdl trigger rumble")
      && lower.contains("not\nadaptive-trigger resistance"),
    "missing simple-vs-adaptive trigger distinction"
  );

  assert!(
    !lower.contains("4 bytes") && !lower.contains("four-byte"),
    "README contains stale four-byte protocol framing claim"
  );
  assert!(
    !lower.contains("every pad has exactly two motors")
      && !lower.contains("every gamepad has exactly two motors")
      && !lower.contains("all pads have exactly two motors"),
    "README claims every SDL pad has exactly two motors"
  );
}
