# CLAUDE.md

Last verified: 2026-05-19

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build                          # Debug build
cargo build --release                # Release build (LTO enabled)
cargo test                           # Run all tests
cargo test -p buttplug_server        # Run tests for specific crate
cargo fmt --all -- --check           # Check formatting
cargo fmt                            # Auto-format (2-space indent, edition 2024)
```

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

**Async Architecture**: Heavy use of tokio channels (mpsc, broadcast, oneshot) for communication between components. Runtime abstraction supports tokio (production) and WASM.

**Device Lifecycle**:
```
Discovery → Identification → Protocol Matching → Connection → User Config → Operation
```

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
Simulated devices allow testing the full device lifecycle without real hardware. Configuration lives in the user config under `simulated_devices`, each entry referencing an archetype from `simulated.yml` (5 archetypes: single-vibe, dual-vibe, rotate, linear, multi-feature). Key contracts:
- `SimulatedSpecifier` variant on `ProtocolCommunicationSpecifier` -- matches devices by archetype name
- `SimulatedDeviceConfigEntry` in `UserConfigDefinition` -- identifier (archetype name), optional display_name, auto-generated UUID address
- `DeviceConfigurationManager::available_simulated_archetypes()` -- lists valid archetypes with feature summaries
- `ServerDeviceManagerBuilder::finish()` auto-wires `SimulatedHardwareCommunicationManager` when simulated_devices is non-empty
- Validation rejects unknown archetypes and duplicate addresses at config build time
- `SimulatedProtocol` is a no-op handler; `SimulatedHardwareConnector` creates in-memory endpoints

## Contributing

**Issues must be filed and discussed before PRs are submitted.** Approval from @qdot required. Non-issue PRs will be closed.

Communication: Discord (discord.buttplug.io), Forums (discuss.buttplug.io), GitHub Issues
