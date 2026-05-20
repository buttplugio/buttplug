# Human Test Plan: Simulated Device Hardware Manager

## Prerequisites

- Rust toolchain installed (stable, edition 2024)
- Repository checked out at branch `simulated-device-hwmgr`
- All automated tests passing:
  ```bash
  cargo test -p buttplug_server_device_config
  cargo test -p buttplug_server
  cargo test -p buttplug_tests test_simulated
  ```

## Phase 1: Code Review of simulated.yml Configuration

| Step | Action | Expected |
|------|--------|----------|
| 1.1 | Open `crates/buttplug_server_device_config/device-config-v4/protocols/simulated.yml` | File exists and is valid YAML |
| 1.2 | Verify 5 configurations are defined under `configurations:` | Identifiers: `simulated-1vibe`, `simulated-2vibe`, `simulated-rotator`, `simulated-oscillator`, `simulated-stroker` |
| 1.3 | For each configuration, check the `id` field and all feature `id` fields | All IDs are explicit UUID literals matching the pattern `a1b2c3d4-XXXX-4000-8000-XXXXXXXXXXXX`. No calls to UUID generation functions. |
| 1.4 | Verify each configuration has the correct feature types: 1vibe has 1 vibrate feature, 2vibe has 2 vibrate features, rotator has 1 rotate feature, oscillator has 1 oscillate feature, stroker has 1 position + 1 hw_position_with_duration feature | Feature types and counts match the descriptions |
| 1.5 | Verify the `communication:` section lists `simulated:` with all 5 archetype names | Names array contains all 5 identifiers |

## Phase 2: Code Review of SimulatedHardwareInternal

| Step | Action | Expected |
|------|--------|----------|
| 2.1 | Open `crates/buttplug_server/src/device/hardware/simulated.rs` | File exists |
| 2.2 | Inspect `read_value()` method | Returns `Ok(HardwareReading::new(endpoint, &[]))` -- empty byte slice, no error |
| 2.3 | Inspect `disconnect()` method | Sends `HardwareEvent::Disconnected(address)` on the broadcast channel, returns `Ok(())` |
| 2.4 | Inspect `write_value()` method | Returns `future::ready(Ok(()))` -- always succeeds, no side effects |

## Phase 3: Code Review of Available Devices API

| Step | Action | Expected |
|------|--------|----------|
| 3.1 | Open `crates/buttplug_server_device_config/src/device_config_manager.rs`, find `available_simulated_archetypes()` | Method exists on `DeviceConfigurationManager` |
| 3.2 | Read the method body | Filters protocol definitions for simulated protocol, extracts identifier, display name, and feature summaries |
| 3.3 | Verify the returned `SimulatedDeviceArchetype` struct includes: `identifier`, `display_name`, `output_features` | All three fields are present and populated from the config |

## Phase 4: Protocol Handler Review

| Step | Action | Expected |
|------|--------|----------|
| 4.1 | Open `crates/buttplug_server/src/device/protocol_impl/simulated.rs` | File exists |
| 4.2 | Verify `generic_protocol_setup!(SimulatedProtocol, "simulated")` macro invocation | Macro registers the protocol with name "simulated" matching the YAML config |
| 4.3 | Verify all `handle_output_*_cmd` methods return `Ok(vec![])` | No hardware commands are emitted (no-op), correct for simulated devices |

## End-to-End: Simulated Device Discovery and Command Flow

1. Run `cargo test -p buttplug_tests test_simulated_1vibe_observation -- --nocapture`
2. Confirm the server performs handshake, starts scanning, and receives a DeviceList containing the simulated device
3. Confirm a vibrate command succeeds without error
4. Confirm an OutputObservation is received with `device_index` matching the discovered device, `feature_index=0`, `output_type="Vibrate"`, `value=50.0`

## End-to-End: Multi-Archetype Validation

1. Run `cargo test -p buttplug_tests test_simulated_diverse_archetypes -- --nocapture`
2. Confirm all 5 archetypes are iterated: simulated-1vibe, simulated-2vibe, simulated-rotator, simulated-oscillator, simulated-stroker
3. For each archetype, confirm a server is created, scanning discovers exactly one device, and the DeviceList is non-empty

## End-to-End: Error Rejection Pipeline

1. Run `cargo test -p buttplug_server_device_config test_invalid_archetype_rejected -- --nocapture`
2. Confirm "nonexistent-device" identifier causes `builder.finish()` to return an error
3. Run `cargo test -p buttplug_server_device_config test_duplicate_address_rejected -- --nocapture`
4. Confirm two entries with the same address cause `builder.finish()` to return an error

## Human Verification Required

| Criterion | Why Manual | Steps |
|-----------|------------|-------|
| AC2.3: Stable UUIDs | UUID stability is a design intent constraint | Review `simulated.yml`: confirm all `id` fields are explicit UUID literals with the `a1b2c3d4-XXXX-4000-8000-*` namespace pattern. Verify no UUID generation code exists in the config pipeline. |
| AC6.1-6.2: Available archetypes API | No dedicated runtime test | Review `available_simulated_archetypes()` logic. Recommendation: add a unit test asserting 5 entries with non-empty `output_features`. |

## Traceability

| Acceptance Criterion | Automated Test | Manual Step |
|----------------------|----------------|-------------|
| AC1.1: Simulated variant | Compile-time | -- |
| AC1.2: Serialization roundtrip | `user.rs::test_simulated_device_config_roundtrip` | -- |
| AC1.3: Only SimulatedHWCM matches | `test_simulated_device_appears_on_scan` | -- |
| AC2.1: simulated.yml parsed | Compile-time + config parsing tests | -- |
| AC2.2: 5 archetypes defined | `test_simulated_diverse_archetypes` | -- |
| AC2.3: Stable UUIDs | -- | Phase 1, Steps 1.3-1.4 |
| AC2.4: generic_protocol_setup! | Compile-time | Phase 4, Steps 4.1-4.3 |
| AC3.1-3.2: Manager/Builder traits | Compile-time | -- |
| AC3.3: DeviceFound on scan | `test_simulated_device_appears_on_scan` + unit tests | -- |
| AC3.4: Device list at construction | All integration tests | -- |
| AC3.5: can_scan() | Unit tests (with/without devices) | -- |
| AC4.1-4.6: Hardware implementation | Unit tests (connector, specializer, internal) | Phase 2 |
| AC5.1-5.5: User config | `test_simulated_device_config_roundtrip`, address/display_name tests | -- |
| AC6.1-6.3: Available devices API | Compile-time | Phase 3 |
| AC7.1: Observations emitted | `test_simulated_1vibe_observation` | -- |
| AC7.2: Correct observation fields | 1vibe, 2vibe, rotator observation tests | -- |
| AC7.3: Stop produces zero | `test_simulated_stop_produces_zero_observation` | -- |
| AC8.1: Invalid archetype rejected | `test_invalid_archetype_rejected` | -- |
| AC8.2: Duplicate address rejected | `test_duplicate_address_rejected` | -- |
