# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo test -p buttplug_tests                           # Run all tests in this crate
cargo test -p buttplug_tests test_device_protocols      # Run all device protocol tests
cargo test -p buttplug_tests "Lovense"                  # Run tests matching a pattern (e.g. one protocol)
cargo test -p buttplug_tests test_server                # Run server tests
cargo test -p buttplug_tests test_serializers           # Run serializer tests
```

## What This Crate Is

This is the **integration test crate** for the buttplug workspace. It exists as a separate crate because its tests span multiple library boundaries (`buttplug_core`, `buttplug_client`, `buttplug_server`, `buttplug_client_in_process`, `buttplug_server_device_config`) and cannot live in any single library crate.

There is no `src/` directory — this crate is tests-only. All code lives under `tests/`.

## Test Architecture

### Device Protocol Tests (YAML-driven)

The bulk of this crate is **data-driven device protocol tests** in `tests/test_device_protocols.rs`. Each test is a `#[test_case]` that loads a YAML file from `tests/util/device_test/device_test_case/` and runs it against the server.

YAML test case structure (`DeviceTestCase`):
- `devices` — list of test device identifiers and expected names
- `device_config_file` / `user_device_config_file` — optional custom config overrides
- `device_init` — initialization sequence (subscribe, write handshake bytes, receive notifications)
- `device_commands` — sequence of `Messages` (client commands like Vibrate/Scalar/Stop), `Commands` (expected hardware writes), and `Events` (simulated device notifications)

To add a test for a new device protocol: create a YAML file in the `device_test_case/` directory and add a `#[test_case]` line in `test_device_protocols.rs`.

### Multi-Version Client Testing

Tests run across multiple protocol spec versions (v0–v4) via version-specific client implementations in `tests/util/device_test/client/client_v{0,1,2,3,4}/`. Each version has its own client, connector, serializer, and event loop that speaks that version of the Buttplug protocol. The YAML test runner exercises each test case against all relevant protocol versions.

### Test Utilities (`tests/util/`)

- `mod.rs` — Factory functions: `test_server()`, `test_client()`, `test_client_with_device()`, `test_client_with_device_and_custom_dcm()` for quickly wiring up in-process client-server pairs
- `test_device_manager/` — `TestDeviceCommunicationManagerBuilder` and `TestDevice` that simulate BLE/hardware devices without real hardware. Devices are identified by `TestDeviceIdentifier` (name + optional address)
- `test_server.rs` — `ButtplugTestServer` wrapping a server with a connector event loop (used by WebSocket and remote connector tests)
- `channel_transport.rs` — In-memory transport for testing client-server communication without network
- `delay_device_communication_manager.rs` — A comm manager that introduces delays, used to test scanning-in-progress scenarios

### Other Test Files

- `test_server.rs` — Server handshake, ping, and message validation tests
- `test_server_device.rs` — Server-side device management tests
- `test_client.rs` — Client connection lifecycle tests (currently commented out, pending rework)
- `test_client_device.rs` — Client-side device command tests
- `test_serializers.rs` — Message serialization/deserialization across protocol versions
- `test_message_downgrades.rs` — Protocol version downgrade path tests
- `test_disabled_device_features.rs` — Tests for user config feature disabling
- `test_output_observations.rs` — Integration tests for output observability (observation stream, filtering, multi-device, enable/disable)
- `test_task_lifecycle.rs` — Integration test asserting the global Task Registry has no live tasks under the server's scope after server shutdown (verifies `TaskScope` ownership leaves no leaked tasks)
- `test_websocket_connectors.rs` / `test_websocket_device_comm_manager.rs` — WebSocket transport integration tests
