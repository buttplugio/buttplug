# CLAUDE.md

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

## Contributing

**Issues must be filed and discussed before PRs are submitted.** Approval from @qdot required. Non-issue PRs will be closed.

Communication: Discord (discord.buttplug.io), Forums (discuss.buttplug.io), GitHub Issues
