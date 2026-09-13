# Script Protocol API v1

## Overview

Script protocols are device protocols written in [Rhai](https://rhai.rs/). When enabled, the server loads them from a directory at startup. Each `.rhai` file defines one protocol. A file is compiled once and its protocol is registered alongside the built-in Rust protocols. If a script reports the same protocol name as a built-in protocol, the script **overrides** that built-in. If multiple files report the same script protocol name, the first file in deterministic sorted filename order is kept and the duplicate is skipped.

Loading is fail-soft per file: a script that cannot be read, compiled, or validated is skipped with a warning and loading continues. A missing directory is a no-op. An unreadable path or a path that exists but is not a directory fails server startup.

## Configuring a script directory

The option belongs to the device-manager builder, not to `ButtplugServerBuilder`:

```rust
use std::path::PathBuf;

let mut device_manager = ServerDeviceManagerBuilder::new(device_configuration_manager);
device_manager.script_protocol_directory(PathBuf::from("scripts/protocols"));
```

The `rhai-protocols` cargo feature must be enabled. The directory is scanned when the device manager is finished during server startup. Without that feature, configuring a directory is ignored with a warning.

## Script API contract

This document describes API version 1. Every script must define `metadata()`. Rhai functions return their final expression, so the examples below use an object or array as the final expression.

### Required metadata

```rhai
fn metadata() {
  #{ "protocol": "my-protocol", "api_version": 1 }
}
```

`metadata()` must return an object map containing:

- `protocol`: a non-empty string.
- `api_version`: the integer `1`.

A missing function, wrong return shape, missing field, empty protocol name, or unsupported API version causes that file to be skipped.

### Optional initial state

```rhai
fn init_state() {
  #{ "speeds": [0, 0] }
}
```

`init_state()` is optional and, when present, must return an object map. It is invoked once when the file is loaded. Each device connection receives a deep copy of that result as `this`; state persists across handler calls for that connection and is not shared with other connections. If the function is absent, `this` starts as an empty map.

### Optional command handlers

Handlers receive integer arguments in device units. Values have already been scaled to the feature's configured step count. A missing handler means that command is not implemented: it returns `UnhandledCommand` without panicking, just like a release-mode Rust protocol.

```text
handle_vibrate(index, speed)
handle_oscillate(index, speed)
handle_rotate(index, speed)                 // speed is signed
handle_constrict(index, level)
handle_spray(index, level)
handle_led(index, level)
handle_temperature(index, level)             // level is signed
handle_position(index, position)
handle_position_duration(index, position, duration_ms)
```

### Handler return value

A handler returns an array of command maps. Each command map has these fields:

- `endpoint` (required): a string naming an `Endpoint`, using its serde name, such as `"tx"`, `"rx"`, or `"command"`. Unknown endpoint strings are errors.
- `data` (required): a `Blob` or an array of integers. Every byte must be strictly in the inclusive range `0..=255`; values are not silently truncated.
- `write_with_response` (optional): a boolean, defaulting to `false`.
- `command_ids` (optional): an array of UUID strings. If omitted, the command uses the UUID of the feature being handled. This can be used for protocols whose writes have a fixed protocol UUID, such as Je Joue's `d3dd2bf5-b029-4bc1-9466-39f82c2e3258`.

For example:

```rhai
[
  #{
    "endpoint": "tx",
    "data": [0x01, 0x02],
    "write_with_response": true,
    "command_ids": ["d3dd2bf5-b029-4bc1-9466-39f82c2e3258"],
  },
]
```

Runtime errors, incorrect return or field shapes, invalid UUIDs, and out-of-range array bytes are reported as device-specific errors. Script runtime errors use the stable prefix `Rhai protocol <name>: `, followed by the Rhai error text and source position when available. Script execution never panics or hangs. (Invalid endpoint names are rejected as invalid endpoints.)

## Language subset and limits

Scripts use core Rhai only: integers, arithmetic and bit operations, arrays, maps, `Blob`, strings, and control flow. `import` and `eval` are disabled and rejected at parse time. Module resolution is a dummy resolver, so scripts cannot load files. The std package's `sleep` function is replaced with an immediate error — it would block a server thread without consuming any of the operation budget below. `print` and `debug` output is routed into the server log rather than written to stdout.

The shared engine enforces these limits for each call:

- Maximum 1,000,000 operations.
- Maximum call depth of 64.
- Maximum 1024 defined functions and 1024 live variables.
- Maximum 4096 entries in an array.
- Maximum 4096 entries in an object map.
- Maximum string length of 65,536 characters.

Loading is also bounded: a single script file may be at most 1 MiB, and at most 256 script files are loaded from one directory (excess files are skipped with a reason). Exceeding a runtime limit terminates the call with an error.

`init_state()` templates may only contain integers, floats, bools, chars, strings, Blobs, arrays, and maps; anything else (such as function pointers) is rejected at load time so that each device connection's state copy is always fully independent.

## Worked example: `maxpro.rhai`

The following is the shipped `crates/buttplug_server/scripts/protocols/maxpro.rhai` example, verbatim:

```rhai
// MaxPro 2 protocol script.
//
// Port of crates/buttplug_server/src/device/protocol_impl/maxpro.rs:
// single-motor vibration with a trailing checksum byte (wrapping sum of the
// first nine bytes).

fn metadata() {
  #{ "protocol": "maxpro", "api_version": 1 }
}

fn handle_vibrate(index, speed) {
  let data = [
    0x55,
    0x04,
    0x07,
    0xff,
    0xff,
    0x3f,
    speed & 0xff,
    0x5f,
    speed & 0xff,
    0x00,
  ];
  let crc = 0;
  for b in data {
    crc = (crc + b) & 0xff;
  }
  data[9] = crc;

  [
    #{
      "endpoint": "tx",
      "data": data,
    },
  ]
}
```

The shipped `aneros.rhai` is a stateless example: its `handle_vibrate` creates one `tx` packet per motor. The shipped `jejoue.rhai` is stateful: `init_state()` creates `this.speeds`, and `handle_vibrate` updates that state while assigning every write Je Joue's fixed command UUID.

## Status

This is phase 1 of script protocol support. Keepalive, scripted initialization, subscriptions, and battery scripting are not available yet. Scripts coexist with Rust protocols and override them only when a configured script successfully loads with the same protocol name.
