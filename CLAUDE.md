# CLAUDE.md

Last verified: 2026-06-09

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build (LTO enabled)
cargo test                           # Run all tests
cargo test -p buttplug_server        # Run tests for specific crate
cargo +nightly fmt --all -- --check  # Check formatting (MUST use nightly)
cargo +nightly fmt                   # Auto-format (2-space indent, edition 2024)
```

**Formatting gotcha**: rustfmt.toml uses nightly-only options (`imports_layout`, `empty_item_single_line`). Running `cargo fmt` on the STABLE toolchain silently ignores them and rewrites the entire workspace into the wrong style (~190 files of import-collapsing churn). Always use `cargo +nightly fmt`. CI checks formatting with nightly.

**Linux dependencies**: `libudev-dev`, `libusb-1.0-0-dev` (for serial/HID support)

**WASM build**:
```bash
wasm-pack build --dev crates/buttplug_server --no-default-features --features wasm
```

## Architecture Overview

Buttplug is a framework for interfacing with intimate hardware devices. It uses a client-server architecture where:

- **Clients** send commands (vibrate, rotate, etc.) through a connector
- **Servers** translate commands to device-specific protocols and manage hardware

### Crate Organization

**Core Libraries:**
- `buttplug_core` - Protocol messages, errors, shared types
- `buttplug_client` - Client API for connecting to servers
- `buttplug_server` - Server implementation, device management, 115+ device protocols
- `buttplug_client_in_process` - Integrated client+server for standalone apps

**Hardware Managers** (under `buttplug_server_hwmgr_*`):
- `btleplug` - Bluetooth LE (primary, cross-platform)
- `serial`, `hid` - USB serial and HID devices
- `lovense_dongle`, `lovense_connect` - Lovense-specific (deprecated)
- `xinput` - Windows gamepad vibration
- `websocket` - WebSocket device forwarders
- `simulated` - In-process simulated devices (no real hardware; lives in `buttplug_server`)

**Infrastructure:**
- `buttplug_server_device_config` - Device configuration database
- `buttplug_transport_websocket_tungstenite` - WebSocket transport
- `intiface_engine` - CLI frontend for running servers

### Key Patterns

**Message-Based Protocol**: All client-server communication uses versioned JSON messages (v0-v4). Version negotiation happens during handshake.

**Async Architecture**: Heavy use of tokio channels (mpsc, broadcast, oneshot) for communication between components. Runtime abstraction supports tokio (production) and WASM. Spawned tasks are owned by `TaskScope`s rather than fire-and-forget `spawn!` (see Task Lifecycle below).

**Device Lifecycle**:
```
Scanning → Identification → Connection → Configuration → Operation
```
Identification and protocol matching are a single step — a device is identified *via* its protocol's specifiers.

**Server Connection State Machine**:
```
AwaitingHandshake → Connected { client_name, spec_version }
                  → Disconnected
                  → PingedOut
```

**Trait-Based Device Abstraction**:
- `ButtplugProtocol` - Device capability definitions
- `Hardware` - Device communication interface
- `DeviceCommunicationManager` - Hardware discovery
- `ServerDeviceManager` - Orchestrates devices and protocols

**Output Observability** (opt-in):
When `emit_output_observations` is enabled, the server broadcasts `OutputObservation` events for every output command sent to a device. The data flows through broadcast channels:
```
DeviceHandle → ServerDeviceManager → ButtplugServer::output_observation_stream()
             → ButtplugRemoteServer (as ButtplugRemoteServerEvent::OutputObservation)
             → Frontend (as EngineMessage::DeviceOutputObservation)
```
Each observation carries `device_index`, `feature_index`, `output_type`, and `value`. Disabled by default to avoid overhead; enable via `ServerDeviceManagerBuilder::emit_output_observations(true)` or `EngineOptions::emit_output_observations`.

**Simulated Devices** (no-hardware testing):
Simulated devices allow testing the full device lifecycle without real hardware. Configuration lives in the user config under `simulated_devices`, each entry referencing an archetype from `simulated.yml` (5 archetypes: simulated-1vibe, simulated-2vibe, simulated-rotator, simulated-oscillator, simulated-stroker). Key contracts:
- `SimulatedSpecifier` variant on `ProtocolCommunicationSpecifier` -- matches devices by archetype name
- `SimulatedDeviceConfigEntry` in `UserConfigDefinition` -- identifier (archetype name), optional display_name, auto-generated UUID address
- `DeviceConfigurationManager::available_simulated_archetypes()` -- lists valid archetypes with feature summaries
- `ServerDeviceManagerBuilder::finish()` auto-wires `SimulatedHardwareCommunicationManager` when simulated_devices is non-empty
- Validation rejects unknown archetypes and duplicate addresses at config build time
- `SimulatedProtocol` is a no-op handler; `SimulatedHardwareConnector` creates in-memory endpoints

**Task Lifecycle** (`buttplug_core::util::task`):
Spawned async tasks are owned by a `TaskScope` ownership tree rather than fire-and-forget, giving cooperative cancellation and global introspection. Key contracts:
- `TaskScope::root(name)` makes a root scope (path gets a unique numeric suffix, e.g. `server-2`, so parallel instances don't collide); `.child(name)` derives a sub-scope whose token is a child of the parent's.
- `scope.spawn(name, |token| async ...)` spawns a task owned by the scope; long-running tasks MUST `select!` on the passed token. Cancelling or dropping a scope cancels its whole subtree.
- `scope.spawn_and_hold(name, ...)` consumes the scope into the task, so drop-cancel can't fire before the task runs (used for `FnOnce` callbacks like ping-timeout and protocol subscription handlers).
- `scope.shutdown().await` cancels the subtree and waits until every task under it has deregistered (wrap in a timeout if tasks may be uncooperative).
- `spawn_detached(name, fut)` is a rare escape hatch: registered under `detached/{name}` but uncancellable. Prefer scopes.
- `TaskRegistry` (global, via `registry()`) records every live task: `snapshot()`, `live_count_under(prefix)` (segment-aware prefix match), `event_stream()` (`TaskEvent::Started`/`Ended`), `wait_empty_under(prefix)`.
- Ownership in the server: `ButtplugServer` owns a `server` root scope; `ServerDeviceManager` owns a `device-manager` root scope with per-device child scopes (io/event-forwarding/bringup); `PingTimer` is scope-owned (the old `PingMessage::End` + Drop-spawn shutdown hack is gone). `ProtocolHandler::handle_input_subscribe_cmd` now takes a `TaskScope` param.
- `intiface_engine` exposes this to frontends when `emit_task_events` is set: registry events forward as `EngineMessage::TaskStarted`/`TaskEnded`, and `IntifaceMessage::RequestTaskList` returns `EngineMessage::TaskList` (`Vec<TaskListEntry>`).

## Agent skills

### Issue tracker

GitHub Issues on `buttplugio/buttplug` via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary (needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout — one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Contributing

**Issues must be filed and discussed before PRs are submitted.** Approval from @qdot required. Non-issue PRs will be closed.

Communication: Discord (discord.buttplug.io), Forums (discuss.buttplug.io), GitHub Issues
