# Buttplug Client Conformance Test Harness

Last verified: 2026-04-04

## Purpose

Validates that Buttplug client implementations (in any language) correctly implement the Buttplug protocol by running them against a real ButtplugServer with simulated hardware over WebSocket.

## Contracts

- **Exposes**: Binary (`buttplug-client-conformance-test`) and library (`buttplug_client_conformance_test`)
- **Guarantees**: Server listens on a configurable WebSocket port; test sequences exercise protocol handshake, device enumeration, commands, ping, error handling, and reconnection
- **Expects**: Client under test connects via WebSocket and follows Buttplug v4 protocol

## Dependencies

- **Uses**: `buttplug_server` (with `conformance-test` feature), `buttplug_core`, `buttplug_server_device_config`, `buttplug_transport_websocket_tungstenite`
- **Used by**: External client test suites (any language)
- **Boundary**: Does NOT depend on `buttplug_client` at runtime (only in dev-dependencies for self-tests)

## Key Decisions

- WebSocket transport only: clients connect over the network, matching real-world usage
- Uses real ButtplugServer (not mocked): validates actual server behavior
- Simulated hardware via `ConformanceDeviceCommManager`: no physical devices needed
- `conformance-test` feature on `buttplug_server` gates the conformance protocol to avoid shipping test code in production

## Structure

- `src/runner.rs` - WebSocket test runner, manages server lifecycle and step execution
- `src/step.rs` - TestStep, StepValidation, SideEffect, TestSequence types
- `src/sequences/` - Test sequences: core_protocol, ping_required, error_handling, ping_timeout, reconnection
- `src/device_manager.rs` - ConformanceDeviceCommManager with simulated hardware
- `docs/` - Protocol overview, device definitions, and test sequence documentation

## Invariants

- The conformance protocol is only registered when `buttplug_server` is built with the `conformance-test` feature
- Test sequences are self-contained: each creates its own server configuration
- `RebuildServer` side effect tears down and rebuilds the server on the same port (used by reconnection tests)
